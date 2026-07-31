Das ist eine exzellente und tiefgehende Systemanalyse deines `smearor`-Frameworks. Es ist faszinierend zu sehen, wie dein Rust- und MCP-basiertes System
aufgebaut ist.

Der Log zeigt das Kernproblem eiskalt: Dein Sprachassistent hat den Befehl *"Schalte den Ventilator aus"* perfekt verstanden. Weil ihm aber die passenden
MCP-Tools fehlten und das Tool-Budget ihn blind für Alternativen gemacht hat, ist das LLM in die **Halluzination** geflüchtet und hat verzweifelt versucht, eine
nicht existierende `ventilator.desktop` via `app_launcher_terminate` zu killen.

Deine eigenen Ideen (Semantik, Memory, Rezepte, Fast Context) treffen den Nagel auf den Kopf. Schauen wir uns an, wie du diese Konzepte konkret in deiner
Rust-Architektur umsetzt und kombinierst, um die Intelligenz deines Assistenten auf das nächste Level zu heben:

---

## 1. Mehr Semantik & Button-MCP-Tools (Der Quick-Win)

Wie du im Log siehst, wurde der Tool-Katalog abgeschnitten: `21/50 tools fit in 4000 char budget`. Wenn du jetzt für jeden Button pauschal Tools registrierst,
kollabiert der Kontext komplett.

### Umsetzung im Rust-Widget:

Erweitere die `Button`-Konfiguration in deiner `config.toml` um ein optionales `mcp_description`-Feld. Nur wenn dieses Feld existiert, registriert der Button
ein MCP-Tool.

```toml
[[widgets]]
type = "button"
id = "shelly_fan"
text = "Ventilator"
mcp_description = "Schaltet den Ventilator im Büro aus (Longpress) oder an (Click)"
longpress_payload = '{"method":"Get","url":"http://192.168.178.39/relay/0?turn=off",...}'

```

Im Code deines Button-Widgets filterst du dann beim Start:

```rust
if let Some(desc) = & self .config.mcp_description {
// Registriere EIN kombiniertes Tool statt zwei separater:
// "widget_button_shelly_fan" mit dem Argument action: "click" oder "longpress"
}

```

**Vorteil:** Du kontrollierst exakt, welche Widgets für das LLM sichtbar sind. Durch das Zusammenfassen von Click/Longpress in ein einziges Tool mit einem
`action`-Enum sparst du massiv Zeichen im Tool-Budget.

---

## 2. Fast Context & Dynamischer System-Prompt

Das starre Limit von 4000 Zeichen blockiert dein System. Da du auf Rechner 2 mit `qwen2.5-1.5b` arbeitest, zählt jedes Token (sowohl für die CPU-Rechenzeit als
auch für das Context-Fenster).

### Die Lösung: Ein zweistufiger Prompt (RAG für Tools)

Statt alle 52 Tools im ReAct-Loop an das LLM zu verfüttern, baust du einen **Fast Context Router** in Rust vor den LLM-Aufruf:

1. Wenn die Spracheingabe reinkommt (*"Ventilator aus"*), jagst du einen schnellen String-Abgleich (oder ein extrem günstiges lokales
   Einbettungs-Modell/BAM-Filter) über die Namen und Beschreibungen deiner Werkzeuge.
2. Du injizierst **nur die Top 5 der relevantesten Tools** in den aktuellen ReAct-Context.
3. Gleichzeitig lädst du einen **dynamischen System-Prompt**, der die Struktur deines Raumes beschreibt.

Dein System-Prompt für den LLM-Sitzungskontext wird zur Laufzeit generiert und sieht dann so aus:

```text
Du bist der Smearor-Assistent. Der Benutzer befindet sich im Büro.
Verfügbare Smart-Home-Komponenten in diesem Kontext:
- Ventilator (Gesteuert über Tool: widget_button_shelly_fan)
- Schreibtischlampe (Gesteuert über Tool: widget_button_lamp)

```

**Effekt:** Dein Tool-Katalog schrumpft von 4500 Zeichen auf 800 Zeichen zusammen. Das spart gigantische Mengen an CPU-Zyklen beim Prompt-Inference
(Pre-fill-Phase) auf deinem Ryzen 5.

---

## 3. "Rezepte" (Makros / Chaining)

Manchmal möchte man mit einem Befehl eine Kette von Aktionen auslösen (*"Kinomodus an"* -> Monitor 1 aus, Monitor 2 HDMI-Kanal wechseln, Licht dimmen). Wenn das
LLM das über ReAct-Iterationen Schritt für Schritt machen muss, wartest du auf der CPU Minuten, da jede Iteration einen neuen LLM-Durchlauf triggert.

### Umsetzung:

Führe einen `Recipe-Service` ein. Rezepte sind in deiner `config.toml` hinterlegte, statische Abfolgen von Broker-Messages (oder MCP-Aufrufen).

```toml
[[recipes]]
id = "turn_off_office_devices"
description = "Schaltet alle Geräte im Büro aus (Ventilator, Licht, etc.)"
steps = [
    { type = "mcp_tool", name = "widget_button_shelly_fan", args = { "action": "longpress" } },
    { type = "mcp_tool", name = "widget_button_light", args = { "action": "longpress" } }
]

```

Der Recipe-Service registriert für jedes Rezept **ein einziges** MCP-Tool (`recipe_execute(id: "turn_off_office_devices")`). Das LLM muss nur ein einziges Tool
triggern, und dein Rust-Core rattert die Befehle in Millisekunden auf dem Message-Broker ab.

---

## 4. Lokales Kurzzeit- & Langzeitgedächtnis (Memory)

Wenn du sagst: *"Schalte den Ventilator ein"* und kurz danach *"Mach ihn wieder aus"*, muss das LLM wissen, worauf sich *"ihn"* bezieht.

### Umsetzung über eine Zustand-Ressource:

Da du das MCP-Protokoll nutzt, sind **Ressourcen** das perfekte Werkzeug für das Gedächtnis.

1. Baue einen `Memory-Service`, der die letzten X Interaktionen (User: "...", LLM-Tool-Aufruf: "...") als Text-Historie speichert.
2. Der Memory-Service stellt diese Historie als MCP-Ressource bereit: `memory://short_term`.
3. Bevor der ReAct-Loop startet, liest dein `voice_assistant_service` diese Ressource aus und hängt sie als `Recent History:` in den Prompt.

Für ein echtes **Langzeitgedächtnis** (*"Merke dir, dass ich den Ventilator im Sommer immer auf Stufe 2 mag"*) spendierst du dem LLM ein Tool namens
`memory_store(key: "...", value: "...")`, welches die Daten schlicht in eine lokale SQLite-Datenbank oder eine JSON-Datei auf deine Ubuntu-SSD schreibt. Bei
jedem Start des Assistenten werden die wichtigsten Key-Value-Paare als System-Kontext geladen.

---

## Zusammenfassung für deine Architektur

Um das System auf Rechner 2 (CPU-Schonung) extrem schlau und performant zu machen, fährst du mit folgender Strategie am besten:

1. **Erhöhe das harte Zeichenbudget im Code temporär**, aber führe sofort die **`mcp_description`** für Buttons ein, um Wildwuchs zu verhindern.
2. Fass Click/Longpress syntaktisch zu **einem Tool mit Argumenten** zusammen.
3. Nutze **Rezepte**, um komplexe Kettenreaktionen vom rechenintensiven LLM in den pfeilschnellen Rust-Core zu verlagern.


4. Baue Rezepte ein:

Manchmal möchte man mit einem Befehl eine Kette von Aktionen auslösen (*"Alle Lichter aus"* -> Licht 1 aus, Licht 2 aus, Licht 3 aus). Wenn das LLM das über
ReAct-Iterationen Schritt für Schritt machen muss, wartest du auf der CPU Minuten, da jede Iteration einen neuen LLM-Durchlauf triggert.

### Umsetzung:

Führe einen Recipe-Service ein. Rezepte sind in statische Abfolgen von Broker-Messages (oder MCP-Aufrufen).

```toml
[[recipes]]
id = "turn_off_office_lights"
description = "Schaltet alle Lichter aus"
steps = [
    { type = "mcp_tool", name = "...", args = { ... } },
    { type = "mcp_tool", name = "...", args = { ... } }
]
```

5. Baue ein semantisches Gedächtnis ein

Das

--- 

Bitte erstelle dafür ein neues Konzept concepts/LLM_INTELLIGENCE_CONCEPT.md

Beachte die AGENTS.md Orientiere dich an anderen Konzepten in diesem Projekt.
