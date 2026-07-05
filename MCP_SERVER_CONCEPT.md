# MCP-Server Konzept für Smearor Swipe Launcher

Dieses Dokument beschreibt das Konzept für einen **MCP-Server (Model Context Protocol)**, der den *Smearor Swipe Launcher* über eine standardisierte
Schnittstelle für externe KI-Clients verfügbar macht. Der MCP-Server läuft als integrierter Bestandteil der Launcher-Anwendung und kommuniziert ausschließlich
über **SSE (Server-Sent Events)** mit dem MCP-Client.

---

## 1. Ziel & Motivation

Der *Smearor Swipe Launcher* verfügt über einen zentralen Event-Broker, Areas (Widgets als Fenster/Popups), Services und Widgets. Aktuell ist die Steuerung und
Abfrage dieser Komponenten auf die interne Anwendung beschränkt. Ein MCP-Server ermöglicht es KI-Assistenten und externen Tools, den Launcher direkt zu steuern
und Systemzustände abzufragen, ohne proprietäre Schnittstellen zu kennen.

**Vorteile:**

* **Standardisierte KI-Integration:** Jeder MCP-Client (z. B. Claude, Cursor, etc.) kann den Launcher steuern.
* **Area-Automatisierung:** KI-Clients können Launcher-Areas gezielt öffnen und schließen (z. B. „Öffne das Audio-Menü").
* **Broker-Kontrolle:** Nachrichten können auf Topics gesendet werden, um Widgets und Services auszulösen.
* **Plugin-Tools:** Services können semantisch spezifische Tools (z. B. Lautstärke ändern) direkt über den MCP-Server bereitstellen.
* **Status-Abfragen:** Systemwerte wie Uptime, Lautstärke oder Mediaplayer-Status können als Ressourcen abgefragt werden.

---

## 2. Architektur

Der MCP-Server wird als separates Crate `mcp-server` im Workspace implementiert oder als Feature-Flag in der Hauptanwendung `smearor-swipe-launcher`aktivierbar.
Er greift auf denselben internen Zustand zu wie die GTK-Anwendung. Der Transport erfolgt ausschließlich über **SSE (Server-Sent Events)**.

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP CLIENT                              │
│  (z. B. Claude Desktop, Cursor, VS Code Extension)          │
└─────────────────────┬───────────────────────┬───────────────┘
                      │ JSON-RPC / MCP over SSE │
                      ▼                         ▼
┌──────────────────────────────────────────┐  ┌────────────────────────┐
│             MCP-Server (SSE)             │  │  Resource/Tool Registry │
│  ┌────────────────────────────────────┐  │  │  (AreaManager +        │
│  │   Tools: open_area, close_area,    │  │  │   Plugin-Handlers)     │
│  │   send_message, ...                  │  │  └────────────────────────┘
│  └────────────────────────────────────┘  │
└─────────────────────┬────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                 Smearor Swipe Launcher Core                  │
│  ┌─────────────────────┐    ┌─────────────────────────────┐│
│  │   Area Manager      │    │   Central Message Broker      ││
│  │   (open/close/     │    │   (publish/subscribe)         ││
│  │    list areas)      │    │                               ││
│  └─────────────────────┘    └─────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Vorgeschlagene Tools

Tools sind vom MCP-Client aufrufbare Funktionen, die Aktionen im Launcher auslösen.

### 3.1 Pflicht-Tools (ab dem MVP)

| Tool           | Beschreibung                                                                                          | Parameter                                                       |
|----------------|-------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------|
| `open_area`    | Öffnet eine definierte Area anhand ihrer ID.                                                          | `area_id: string`                                               |
| `close_area`   | Schließt eine geöffnete Area.                                                                         | `area_id: string`                                               |
| `list_areas`   | Listet alle konfigurierten Areas mit ID, Position, Aktivierungsstatus und aktueller Sichtbarkeit auf. | –                                                               |
| `focus_area`   | Setzt den Fokus auf eine Area (z. B. für Tastatur-Navigation).                                        | `area_id: string`                                               |
| `send_message` | Sendet eine Nachricht auf ein Topic im zentralen Broker.                                              | `topic: string`, `payload: json`, `target_instance_id?: string` |

### 3.2 Zusätzliche sinnvolle Tools

| Tool               | Beschreibung                                                                                    | Parameter                                             |
|--------------------|-------------------------------------------------------------------------------------------------|-------------------------------------------------------|
| `toggle_area`      | Wechselt den Sichtbarkeitsstatus einer Area.                                                    | `area_id: string`                                     |
| `reload_config`    | Lädt die Konfigurationsdateien (`config.toml`, `services.toml`) neu.                            | –                                                     |
| `get_area_config`  | Gibt die Konfiguration einer Area als JSON zurück.                                              | `area_id: string`                                     |
| `send_action`      | Sendet eine typisierte Action an ein Widget oder einen Service (z. B. `AppLaunch`, `VolumeUp`). | `plugin_id: string`, `action: string`, `params: json` |
| `trigger_widget`   | Löst das primäre Interaktionsereignis eines Widgets aus (z. B. Tap).                            | `widget_id: string`, `event: string`                  |
| `set_global_theme` | Wechselt das globale CSS-Theme (z. B. hell/dunkel).                                             | `theme: string`                                       |
| `play_sound`       | Spielt ein konfiguriertes Systemgeräusch ab.                                                    | `sound_id: string`                                    |

### 3.3 Plugin-bereitgestellte Tools

Neben generischen Core-Tools können Service-Plugins über die **Plugin-Tool-Registry** eigene, semantisch typisierte Tools registrieren. Der MCP-Server fragt
diese Registry ab und stellt die Tools dynamisch bereit.

**Beispiel Audio-Service (basiert auf `PulseCommand`):**

| Tool                           | Beschreibung                                               | Parameter                 |
|--------------------------------|------------------------------------------------------------|---------------------------|
| `plugin.audio.volume_up`       | Erhöht die Lautstärke um einen konfigurierten Schritt.     | –                         |
| `plugin.audio.volume_down`     | Verringert die Lautstärke um einen konfigurierten Schritt. | –                         |
| `plugin.audio.set_volume`      | Setzt die Lautstärke auf einen absoluten Wert.             | `volume: f32` (0.0 – 1.0) |
| `plugin.audio.toggle_mute`     | Wechselt den Mute-Status.                                  | –                         |
| `plugin.audio.mute`            | Stummschaltung aktivieren.                                 | –                         |
| `plugin.audio.unmute`          | Stummschaltung aufheben.                                   | –                         |
| `plugin.audio.next_device`     | Nächstes Audio-Gerät auswählen.                            | –                         |
| `plugin.audio.previous_device` | Vorheriges Audio-Gerät auswählen.                          | –                         |
| `plugin.audio.refresh_status`  | Status manuell neu einlesen.                               | –                         |

Weitere Plugins (z. B. `mpris`, `hyprland`) registrieren analog ihre eigenen Tools, z. B. `plugin.mpris.play_pause`, `plugin.hyprland.switch_workspace`.

---

## 4. Vorgeschlagene Resources

Ressourcen sind vom MCP-Client abfragbare Werte, die den aktuellen Zustand des Launchers oder des Systems widerspiegeln.

### 4.1 Pflicht-Ressourcen (ab dem MVP)

Der `AreaManager` und **jedes Service-Plugin** müssen mindestens ihre zentralen Zustandsressourcen über die Plugin-Resource-Registry registrieren. Der
MCP-Server bietet diese Ressourcen dann dynamisch an.

#### Core-Resources (AreaManager)

| URI                      | Beschreibung                                                   | Format |
|--------------------------|----------------------------------------------------------------|--------|
| `area://list`            | Liste aller konfigurierten Areas mit Status und Position.      | JSON   |
| `area://<area_id>/state` | Aktueller Zustand einer Area (geöffnet, fokussiert, sichtbar). | JSON   |
| `area://current/focus`   | Aktuell fokussierte Area.                                      | JSON   |
| `area://current/visible` | Aktuell sichtbare Area(n).                                     | JSON   |

#### Service-Plugin-Resources

Jedes Service-Plugin bietet eine **Snapshot-Resource** für den kompletten Status sowie, wo sinnvoll, **feingranulare Einzelresources** für häufig abgefragte
Werte.

| Service         | URI                                  | Beschreibung                                                            | Quell-Typ                        |
|-----------------|--------------------------------------|-------------------------------------------------------------------------|----------------------------------|
| `app_launcher`  | `plugin://app_launcher/running_apps` | Status aller überwachten `.desktop`-Dateien (läuft / gestoppt).         | `DesktopFileStatusMessageStabby` |
| `audio`         | `plugin://audio/status`              | Kompletter Audio-Status (Lautstärke, Mute, Geräte, aktives Gerät).      | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/volume`              | Aktuelle Lautstärke (0.0 – 1.0).                                        | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/muted`               | Aktueller Mute-Status.                                                  | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/active_sink`         | Aktives Ausgabegerät mit Name, Index und Kanälen.                       | `AudioStatusMessage`             |
| `audio`         | `plugin://audio/sinks`               | Liste aller verfügbaren Ausgabegeräte.                                  | `AudioStatusMessage`             |
| `mpris`         | `plugin://mpris/status`              | Aktive Player, Wiedergabestatus, Metadaten, Position, Lautstärke.       | `MprisStatusMessage`             |
| `notifications` | `plugin://notifications/status`      | Do-Not-Disturb, aktive Benachrichtigungen, ungelesene Anzahl.           | `NotificationStatusMessage`      |
| `sysinfo`       | `plugin://sysinfo/cpu`               | CPU-Auslastung und -Temperatur.                                         | `CpuStatusMessage`               |
| `sysinfo`       | `plugin://sysinfo/memory`            | RAM-Nutzung, gesamt, belegt, verfügbar.                                 | `MemoryStatusMessage`            |
| `sysinfo`       | `plugin://sysinfo/battery`           | Akkuladestand und Ladezustand.                                          | `BatteryStatusMessage`           |
| `sysinfo`       | `plugin://sysinfo/disks`             | Mountpoint-Nutzung, Lese-/Schreib-Throughput.                           | `DisksStatusMessage`             |
| `sysinfo`       | `plugin://sysinfo/network`           | Ein-/Ausgehende Netzwerk-Throughput.                                    | `NetworkStatusMessage`           |
| `sysinfo`       | `plugin://sysinfo/uptime`            | Uptime in Sekunden und Load-Average.                                    | `UptimeStatusMessage`            |
| `hyprland`      | `plugin://hyprland/active_workspace` | Aktueller Workspace und Fensterliste (neu zu implementieren).           | Eigenes Status-Message           |
| `http`          | `plugin://http/stats`                | Letzte Anfrage-Statistiken oder letzte Antwort (neu zu implementieren). | Eigenes Status-Message           |

### 4.2 Zusätzliche sinnvolle Ressourcen

| URI                   | Beschreibung                           | Format    |
|-----------------------|----------------------------------------|-----------|
| `launcher://config`   | Gesamte aktive Launcher-Konfiguration. | JSON/TOML |
| `launcher://version`  | Version der Launcher-Anwendung.        | JSON      |
| `network://status`    | Netzwerkstatus (verbunden, SSID, IP).  | JSON      |
| `bluetooth://devices` | Gekoppelte Bluetooth-Geräte.           | JSON      |

---

## 5. Tool-Implementierungsdetails (MVP)

### 5.1 `open_area`

```json
{
  "name": "open_area",
  "description": "Opens a Smearor area by its configured ID.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Intern wird dieselbe Funktion aufgerufen wie beim Swipe-Event oder Hotkey: `AreaManager::open(area_id)`.

### 5.2 `close_area`

```json
{
  "name": "close_area",
  "description": "Closes a currently visible Smearor area.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Intern: `AreaManager::close(area_id)`.

### 5.3 `list_areas`

```json
{
  "name": "list_areas",
  "description": "Lists all configured Smearor areas with their current visibility and position.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

Intern: `AreaManager::list()` liefert die konfigurierten Areas mit ihrem aktuellen Zustand zurück.

### 5.4 `focus_area`

```json
{
  "name": "focus_area",
  "description": "Focuses a Smearor area for keyboard navigation.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "area_id": {
        "type": "string",
        "description": "Unique area identifier from config.toml"
      }
    },
    "required": [
      "area_id"
    ]
  }
}
```

Intern: `AreaManager::focus(area_id)`.

### 5.5 `send_message`

```json
{
  "name": "send_message",
  "description": "Publishes a message to a topic on the central message broker.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "topic": {
        "type": "string",
        "description": "Broker topic name"
      },
      "payload": {
        "type": "object",
        "description": "JSON payload to publish"
      },
      "target_instance_id": {
        "type": "string",
        "description": "Optional target widget/service instance ID"
      }
    },
    "required": [
      "topic",
      "payload"
    ]
  }
}
```

Intern wird die Nachricht in ein `FfiEnvelope`-äquivalentes internes Format umgewandelt und über `MessageBrokerHandle::send` veröffentlicht. Für JSON-Payloads
wird die JSON-Converter-Registry des Hosts verwendet.

---

## 6. Resource-Implementierungsdetails (MVP)

### 6.1 Area-Status und Current Area

Die Ressourcen `area://list`, `area://<area_id>/state`, `area://current/focus` und `area://current/visible` werden direkt aus dem `AreaManager` gelesen. Der
`AreaManager` registriert sie beim Start über die Plugin-Resource-Registry. Eine Area-Status-Ressource enthält mindestens:

```json
{
  "area_id": "audio",
  "visible": true,
  "focused": false,
  "position": "bottom",
  "active": true
}
```

### 6.2 Service-Plugin-Resources

Jedes Service-Plugin hält seinen aktuellen Zustand vor und registriert die dazugehörigen Ressourcen. Beispiel `plugin://sysinfo/cpu`:

```
URI: plugin://sysinfo/cpu
MIME type: application/json
Body: { "cpu_usage": 12.5, "cpu_temperature": 45.2 }
```

Beispiel `plugin://audio/status`:

```
URI: plugin://audio/status
MIME type: application/json
Body: {
  "volume": 0.8,
  "is_muted": false,
  "active_device": { "id": 1, "name": "Built-in Audio", "is_default": true }
}
```

### 6.3 Generische Ressourcen aus Plugins

Damit zukünftige Plugins selbstständig Ressourcen bereitstellen können, ohne dass der MCP-Server sie explizit kennen muss, wird eine **Plugin-Resource-Registry
** im Core eingeführt:

```
┌─────────────────────────────────────────┐
│            MCP-Server                   │
│   list_resources() / read_resource()    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│      Plugin Resource Registry           │
│  (globaler Registry im Core)            │
│                                         │
│  plugin://sysinfo/cpu  -> PluginHandler   │
│  plugin://audio/volume -> PluginHandler   │
│  plugin://clock/time   -> PluginHandler   │
└─────────────────────────────────────────┘
```

**Mechanismus:**

1. Jedes Plugin (Widget oder Service) kann während der Initialisierung über einen neuen Callback im `FfiCoreContext` Ressourcen registrieren:
    - `resource_uri: stabby::string::String` (z. B. `plugin://sysinfo/cpu`)
    - `metadata: ResourceMetadata` (Name, Beschreibung, MIME-Type)
    - `read_fn: extern "C" fn(...) -> DynFuture<'static, stabby::string::String>`
2. Der MCP-Server fragt die Registry beim Start ab und registriert alle URIs dynamisch beim MCP-Client.
3. Bei `read_resource(plugin://<plugin>/<name>)` ruft der MCP-Server die zugehörige `read_fn` des Plugins auf.
4. Das Plugin liefert JSON als `stabby::string::String` zurück; der MCP-Server leitet es unverändert an den Client weiter.

Dadurch können auch später entwickelte Plugins Ressourcen bereitstellen, ohne dass der MCP-Server oder das Core-Modell angepasst werden müssen. Für
stabby-FFI-Typen konvertiert das Plugin intern über die JSON-Converter-Registry.

**Kein Last-Value-Cache für MCP:** Da der `AreaManager` und alle Service-Plugins ihre Zustände explizit als Ressourcen bereitstellen, wird für das MCP-Interface
**kein `topic://<topic>/last`-Resource** benötigt. Der MCP-Server liest ausschließlich über registrierte Ressourcen-Handler.

Der Message Broker kann weiterhin einen internen Last-Value-Cache für Widgets/Services unterhalten (Late-Subscriber-Initialisierung), aber dieser Cache ist für
den MCP-Server nicht sichtbar und wird nicht als Resource exponiert.

### 6.4 Plugin-Tool-Registry

Analog zur Plugin-Resource-Registry gibt es eine **Plugin-Tool-Registry**, über die Service-Plugins eigene MCP-Tools registrieren. Der MCP-Server erweitert
damit seine Tool-Liste dynamisch, ohne für jedes Plugin manuell Tool-Handler implementieren zu müssen.

**Registrierung pro Plugin:**

- `tool_id: stabby::string::String` (z. B. `plugin.audio.volume_up`)
- `description: stabby::string::String`
- `input_schema: stabby::string::String` (JSON-Schema für die Tool-Parameter)
- `handler: extern "C" fn(...) -> DynFuture<'static, ToolResult>`

**Ablauf:**

1. Der MCP-Server liest beim Start alle registrierten Tools aus der Registry.
2. Für jedes Tool meldet er `name`, `description` und `inputSchema` dynamisch beim MCP-Client an.
3. Bei einem Tool-Aufruf serialisiert der MCP-Server die JSON-Argumente und übergibt sie an den Plugin-Handler.
4. Das Plugin führt die Aktion aus (z. B. `PulseCommand::VolumeUp`) und gibt ein Ergebnis oder eine Fehlermeldung zurück.

**Beispiel JSON-Schema für `plugin.audio.set_volume`:**

```json
{
  "name": "plugin.audio.set_volume",
  "description": "Sets the audio volume to an absolute value.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "volume": {
        "type": "number",
        "minimum": 0.0,
        "maximum": 1.0,
        "description": "Absolute volume level between 0.0 and 1.0"
      }
    },
    "required": [
      "volume"
    ]
  }
}
```

**Empfohlene Kombination:**

- **Generische Core-Tools** (`open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`) für Launcher- und Broker-Steuerung.
- **Plugin-Tools** (`plugin.audio.*`, `plugin.mpris.*`, `plugin.hyprland.*`) für semantisch spezifische Aktionen.

---

## 7. Roadmap

### Phase 1: Foundation (MVP)

* Crate `mcp-server` anlegen oder Feature-Flag in `smearor-swipe-launcher` einführen.
* MCP-Transport implementieren (ausschließlich SSE gemäß MCP-Spezifikation).
* MCP-Server als integrierter Thread im Core starten.
* Tool-Registry mit `open_area`, `close_area`, `list_areas`, `focus_area`, `send_message`.
* Resource-Registry mit `area://list`, `area://<id>/state`, `area://current/focus`, `area://current/visible`.
* `AreaManager` implementiert und registriert seine Core-Resources.
* `send_message` verarbeitet ausschließlich `serde_json`-Payloads; interne Konvertierung in stabby-FFI-Typen erfolgt im MCP-Server.

### Phase 2: Erweiterung Tools

* `toggle_area`, `reload_config`, `get_area_config` implementieren.
* `send_action` und `trigger_widget` für direkte Widget-Steuerung.

### Phase 3: Plugin-Resource-Registry, Service-Resources & Plugin-Tools

* Generische **Plugin-Resource-Registry** und **Plugin-Tool-Registry** im Core definieren und ins `FfiCoreContext` einbinden.
* Folgende Service-Plugins müssen ihre Ressourcen implementieren und registrieren:
    * `sysinfo`: `plugin://sysinfo/cpu`, `plugin://sysinfo/memory`, `plugin://sysinfo/battery`, `plugin://sysinfo/disks`, `plugin://sysinfo/network`,
      `plugin://sysinfo/uptime`
    * `audio`: Snapshot `plugin://audio/status` sowie feingranulare Resources `plugin://audio/volume`, `plugin://audio/muted`, `plugin://audio/active_sink`,
      `plugin://audio/sinks`
    * `mpris`: `plugin://mpris/status`
    * `notifications`: `plugin://notifications/status`
    * `app_launcher`: `plugin://app_launcher/running_apps`
    * `hyprland`: `plugin://hyprland/active_workspace` (neue Status-Erfassung nötig)
    * `http`: `plugin://http/stats` (neue Status-Erfassung nötig)
* Folgende Service-Plugins müssen ihre Tools implementieren und registrieren:
    * `audio`: `plugin.audio.volume_up`, `plugin.audio.volume_down`, `plugin.audio.set_volume`, `plugin.audio.toggle_mute`, `plugin.audio.mute`,
      `plugin.audio.unmute`, `plugin.audio.next_device`, `plugin.audio.previous_device`, `plugin.audio.refresh_status`
    * `mpris`: `plugin.mpris.play_pause`, `plugin.mpris.next`, `plugin.mpris.previous`, `plugin.mpris.stop`
    * `hyprland`: `plugin.hyprland.switch_workspace` (neue Status-Erfassung nötig)
* Bindung an bestehende Services: `network`, `bluetooth` (sofern vorhanden).

### Phase 4: Erweiterung des Protokolls

* Stabilisierung des SSE-Transports (Wiederverbindung, Heartbeat, Multi-Client-Support).
* Sampling/Logging-Unterstützung für MCP-Clients.
* Authentisierung/Autorisierung, falls der Server über Netzwerk erreichbar sein soll.

### Phase 5: Integration & Tests

* CLI-Argument `--mcp-server` in `smearor-swipe-launcher`.
* Unit-Tests für Tool- und Resource-Handler.
* Manuelle Integrationstests mit Claude Desktop und anderen MCP-Clients.

---

## 8. Offene Fragen & Entscheidungen

1. **Transport:** Ausschließlich SSE. STDIO wird nicht unterstützt.
2. **Prozessmodell:** Integriert als eigener Thread im Launcher-Core.
3. **Berechtigungen:** Erst einmal ignorieren; alle Tools sind ohne zusätzliche Zustimmung nutzbar.
4. **Payload-Typen:** `send_message` verarbeitet strikt Standard-JSON (`serde_json`). Erst im MCP-Server wird das JSON in die internen FFI-sicheren Typen
   konvertiert und über den Broker gesendet. Binäre/stabby-Layouts werden nicht direkt vom Client akzeptiert, um die Kompatibilität mit dem MCP-Ökosystem zu
   wahren.
5. **Discovery:** `launcher://config` wird ungefiltert bereitgestellt. Sicherheit wird erst einmal ignoriert.

---

## 9. Abhängigkeiten

* `rust-sdk` des Model Context Protocol oder eigenständige JSON-RPC-Implementierung.
* Zugang zum internen `AreaManager` und `MessageBroker` des Launchers.
* Plugin-Resource-Registry und Plugin-Tool-Registry im Core.
* JSON-Converter-Registry für die Serialisierung von stabby-FFI-Typen nach JSON.

---

*Konzept für die Bereitstellung des Smearor Swipe Launchers als MCP-Server mit Fokus auf Area-Steuerung, Broker-Nachrichten, zentraler Resource-Registry und
Plugin-Tool-Registry.*
