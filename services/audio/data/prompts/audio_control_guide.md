Audio control guide:

Tools:

- audio_volume_up: Increase volume by a configured step
- audio_volume_down: Decrease volume by a configured step
- audio_set_volume: Set volume to an absolute value (0.0 to 1.0)
- audio_toggle_mute: Toggle mute on/off
- audio_mute: Mute the default output sink
- audio_unmute: Unmute the default output sink
- audio_next_device: Switch to the next output device
- audio_previous_device: Switch to the previous output device
- audio_refresh_status: Force a status refresh from PulseAudio

Resources:

- audio://status: Complete audio status (volume, mute, devices)
- audio://volume: Current volume level (0.0 to 1.0)
- audio://muted: Current mute status
- audio://active_sink: Active output device
- audio://sinks: List of all available output devices
