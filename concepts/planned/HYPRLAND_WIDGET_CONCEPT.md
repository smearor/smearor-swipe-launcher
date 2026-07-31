Basierend auf den implementierten Dispatch- und CTL-Handlern im Hyprland Service bieten sich folgende Funktionalitäten für Launcher-Widgets an:

## Bereits implementiert (Workspace Switcher)

- **Workspace wechseln** ([SwitchWorkspaceMessage](cci:2://file:///home/aschaeffer/git/smearor-swipe-launcher/model/workspace/src/switcher.rs:101:0-104:1)) -
  Swipe-Gesten zum Wechseln zwischen Workspaces
- **Workspace erstellen** ([CreateWorkspaceMessage](cci:2://file:///home/aschaeffer/git/smearor-swipe-launcher/model/workspace/src/switcher.rs:141:0-146:1)) -
  Wispen über den Rand hinaus erstellt neuen Workspace
- **Workspace-Snapshot** - Aktuelle Workspace-Liste und aktiver Workspace anzeigen

## Geeignete weitere Widget-Ideen

### Touch-optimierte Fenstersteuerung

- **Fokus wechseln** (`MoveFocus` mit `Direction`) - Wispen um Fenster-Fokus zu ändern (Up/Down/Left/Right)
- **Fenster schließen** (`KillActiveWindow` / `CloseWindow`) - Button um aktives Fenster zu schließen
- **Vollbild umschalten** (`ToggleFullscreen`) - Button für Vollbild-Toggle
- **Floating umschalten** (`ToggleFloating`) - Button um Floating-Modus zu toggeln
- **Fenster zentrieren** (`CenterWindow`) - Button um aktives Fenster zu zentrieren
- **Fenster durchwechseln** (`CycleWindow` mit `CycleDirection`) - Next/Previous durch offene Fenster

### Workspace-spezifisch

- **Workspace per Name springen** (`Workspace` mit `Name`) - Direkte Workspace-Auswahl über Buttons/Dots
- **Fenster zum Workspace verschieben** (`MoveToWorkspace`) - Drag-and-Drop oder Button um aktives Fenster auf anderen Workspace zu verschieben
- **Special Workspace togglen** (`ToggleSpecialWorkspace`) - Button für Special-Workspace (z.B. Scratchpad)

### Layout & Master-Stack

- **Master hinzufügen/entfernen** (`AddMaster` / `RemoveMaster`) - Layout-Steuerung für Tiling
- **Master fokussieren** (`FocusMaster`) - Springe zum Master-Fenster
- **Orientierung ändern** (`OrientationLeft/Right/Top/Bottom/Center/Next/Prev`) - Layout-Rotation für Tisch-Nutzung
- **Split-Ratio ändern** (`ChangeSplitRatio(f32)`) - Slider für Split-Verhältnis

### Monitor-Steuerung

- **Monitor-Fokus wechseln** (`FocusMonitor` mit `Direction/Id/Name`) - Zwischen Monitoren wechseln
- **Workspaces zwischen Monitoren tauschen** (`SwapActiveWorkspaces`) - Workspace-Monitor-Tausch

### System-Steuerung

- **App starten** (`Exec`) - Fallback für beliebige Kommandos
- **Hyprland neu laden** (`Reload`) - Config-Reload Button
- **DPMS togglen** (`ToggleDPMS`) - Bildschirm an/aus
- **Lock Groups** (`LockGroups` mit `LockType`) - Fenstergruppen sperren
- **Toggle Group** (`ToggleGroup`) - Fenster gruppieren
- **Change Group Active** (`ChangeGroupActive`) - Durch Fenstergruppen navigieren

### Notification

- **Hyprland-Notification senden** (`NotifyCommandMessage`) - Benachrichtigung auf dem Bildschirm anzeigen (Icon, Farbe, Dauer, Text)

## Am besten geeignet für den 65" Touch-Tisch

1. **Fenster-Fokus Widget** - Wispen um `MoveFocus` in 4 Richtungen, mit Anzeige des aktiven Fensters
2. **Fenster-Aktionen Widget** - Buttons für `KillActiveWindow`, `ToggleFullscreen`, `ToggleFloating`, `CenterWindow`
3. **Layout-Controller Widget** - `AddMaster`/`RemoveMaster` + `OrientationNext`/`OrientationPrev` + `ChangeSplitRatio` Slider
4. **Special Workspace Button** - `ToggleSpecialWorkspace` als Toggle-Button
5. **Monitor-Switcher Widget** - `FocusMonitor` mit Direction-Pfeilen, ähnlich wie Workspace-Switcher aber für Monitore
6. **Cycle-Window Widget** - `CycleWindow(Next/Previous)` zum Durchblättern aller offenen Fenster