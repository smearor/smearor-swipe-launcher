Für einen lokalen Sprachassistenten (insbesondere im Zusammenspiel mit Whisper für STT und schnellen Antwortzeiten) sind zwei Faktoren entscheidend: die *
*Latenz** (Time-to-First-Token) und die **Generierungsgeschwindigkeit** (Tokens/Sekunde). Ein Sprachassistent fühlt sich nur dann natürlich an, wenn er
innerhalb von Millisekunden antwortet.

Hier sind die optimalen Empfehlungen für deine beiden, sehr unterschiedlichen Linux-Systeme:

---

## Rechner 1: Das High-End-Kraftpaket

* **Specs:** Ryzen 9 9950X3D, Radeon RX 7900 XTX (24GB VRAM), 96GB DDR5 RAM.
* **Strategie:** Dank ROCm/ROCm-Llama.cpp unter Ubuntu kannst du die mächtige 24GB-Grafikkarte voll ausreizen. Du hast genug VRAM, um extrem fähige Modelle
  komplett im Grafikspeicher zu halten, was für rasende Geschwindigkeiten sorgt.

### Empfohlene LLMs (für die Logik)

1. **Llama 3.3 70B (Quantisiert auf Q2_K oder Q3_K_M)**

* *Warum:* Das absolute Flaggschiff unter den Open-Source-Modellen. Eigentlich braucht es mehr VRAM, aber als extrem leichtgewichtige Quantisierung passt es
  (knapp) in deine 24GB VRAM oder nutzt ein wenig deines schnellen 96GB RAMs als Spillover. Es bietet eine enorme Intelligenz für komplexe Aufgaben (z. B. für
  deine MCP-Tools oder Verschachtelungen).


2. **Phi-4 (14B) oder Qwen 2.5 (14B / 32B)**

* *Warum:* **Der "Sweet Spot" für diesen Rechner.** Ein 14B-Modell passt mit einer hohen Quantisierung (Q8_0 oder FP16) *vollständig und mit riesigem Kontext*
  in die 24GB VRAM deiner RX 7900 XTX. Die Antwortgeschwindigkeit ist extrem hoch, während die Fähigkeiten (insbesondere von Microsofts Phi-4) auf dem Niveau
  deutlich größerer Modelle liegen.

### Sprachverarbeitung (STT/TTS)

* **Whisper-rs (STT):** Nutze hier problemlos das **Whisper Large-v3**-Modell. Deine GPU wird die Transkription in Sekundenbruchteilen erledigen.
* **Kokoro-82M (TTS):** Ein extrem leichtgewichtiges, aber phänomenal gut klingendes Text-to-Speech-Modell, das auf deiner Hardware praktisch in Echtzeit
  generiert.

---

## Rechner 2: Der schlanke Office-/Heim-Server

* **Specs:** Ryzen 5 8500G (iGPU), 16GB DDR5 RAM, kein dedizierter VRAM.
* **Strategie:** Da die Grafikeinheit (iGPU) sich den normalen Systemspeicher mit der CPU teilen muss und du insgesamt nur 16GB RAM hast, müssen wir extrem
  haushalten. Ein LLM darf hier maximal 4 bis 6 GB RAM belegen, damit Linux und dein Voice-Assistant-Dienst stabil laufen.

### Empfohlene LLMs (für die Logik)

1. **Llama 3.2 3B (Instruct) oder Gemma 2 2B**

* *Warum:* Diese Modelle sind winzig, erstaunlich smart für ihre Größe und laufen selbst auf Smartphones. In einer Q4_K_M oder Q8_0 Quantisierung verbrauchen
  sie nur ca. 2 bis 3 GB RAM. Da sie so klein sind, erreichen sie auch auf einer CPU/iGPU-Kombination eine flüssige Ausgabegeschwindigkeit, die für einen
  Sprachassistenten schnell genug ist.


2. **Phi-4-mini (3.8B)**

* *Warum:* Microsofts Mini-Variante der Phi-4-Reihe schlägt in Benchmarks selbst ältere 7B- oder 8B-Modelle, benötigt im Q4-Format aber gerade einmal knapp 3 GB
  RAM. Perfekt, um trotz Hardwarelimitierung logische Befehle exakt zu interpretieren.

### Sprachverarbeitung (STT/TTS)

* **Whisper-rs (STT):** Verwende hier das **Whisper Base** oder **Whisper Small** (jeweils die `.en` oder multilingualen Varianten). Alles darüber hinaus würde
  auf der iGPU/CPU zu lange für die Transkription brauchen, was eine unangenehme Gedenksekunde nach dem Sprechen erzeugt.
* **Kokoro-82M (TTS):** Auch hier die beste Wahl, da es so ressourcenschonend ist, dass es selbst den 8500G kaum belastet.

---

## Zusammenfassung für dein Projekt

| Komponente              | Rechner 1 (High-End GPU)                   | Rechner 2 (Low-RAM iGPU)                   |
|-------------------------|--------------------------------------------|--------------------------------------------|
| **Primäres LLM**        | Phi-4 (14B) @ Q8 oder Llama 3.3 (70B) @ Q3 | Phi-4-mini (3.8B) @ Q4 oder Llama 3.2 (3B) |
| **VRAM/RAM-Auslastung** | ~14 - 22 GB (in der GPU)                   | ~2.5 - 4 GB (im System-RAM)                |
| **Whisper-Modell**      | Large-v3                                   | Base oder Small                            |
| **Fokus**               | Maximale Logik, extrem schnelles Parsing   | Hohe Effizienz, reines Befehls-Parsing     |