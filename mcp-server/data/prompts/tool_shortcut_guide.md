Common user requests and their direct tool shortcuts:

Audio:

- 'Lauter' / 'Volume up' → audio_volume_up
- 'Leiser' / 'Volume down' → audio_volume_down
- 'Stumm' / 'Mute' → audio_toggle_mute

MPRIS:

- 'Pause' / 'Play' → mpris_toggle_play_pause
- 'Nächster Titel' / 'Next track' → mpris_next_track
- 'Vorheriger Titel' / 'Previous track' → mpris_previous_track

Power:

- 'Herunterfahren' / 'Shutdown' → system_power_action { action: 'shutdown' }
- 'Neustart' / 'Reboot' → system_power_action { action: 'reboot' }
- 'Sperren' / 'Lock' → system_power_action { action: 'lock' }

Weather:

- 'Wetter' / 'Weather' → weather_get_forecast
- 'Wettervorhersage' / 'Forecast' → weather_get_forecast

Network:

- 'WLAN an' / 'WiFi on' → network_toggle_radio { technology: 'wifi', enabled: true }
- 'WLAN aus' / 'WiFi off' → network_toggle_radio { technology: 'wifi', enabled: false }

Sysinfo:

- 'Systemstatus' / 'System health' → read resources sysinfo://cpu, sysinfo://memory, sysinfo://temperature-components

Launcher:

- 'Öffne <area>' / 'Open <area>' → open_area { area_id: '<area>' }
- 'Schließe <area>' / 'Close <area>' → close_area { area_id: '<area>' }

Use these shortcuts directly instead of listing all tools first. Only fall back to prompts/list or tools/list when the user's request does not match any
shortcut.
