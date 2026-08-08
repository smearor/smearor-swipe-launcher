Direction of Arrival (DoA) sensor guide:

The DoA service interfaces with the ReSpeaker XVF3800 USB microphone array to determine the direction of incoming audio.

Tools:

- doa_get_direction: Returns the current DoA angle (0-359), mapped compass direction (N/E/S/W), and device connection status
- doa_set_poll_interval: Set the polling interval in milliseconds (min: 50, default: 150). Lower values give more responsive updates but increase USB traffic
- doa_reconnect: Force a USB reconnection to the ReSpeaker XVF3800 device. Use this if the device was unplugged and reconnected

Resources:

- doa://status: Current DoA angle, mapped direction, and device connection status

Notes:

- The angle is calibrated with a rotation offset from the service configuration
- Speech detection indicates whether voice activity is currently detected by the DSP
- If the device is disconnected, use doa_reconnect to re-establish the USB connection
