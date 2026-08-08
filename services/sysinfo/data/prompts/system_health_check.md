System health check guide:

Steps:

1. Read sysinfo://cpu for current CPU usage and temperature.
2. Read sysinfo://memory for RAM usage.
3. Read sysinfo://temperature-components for detailed thermal sensor data.
4. Read sysinfo://battery for battery level and charging state.
5. Read sysinfo://uptime for system uptime and load average.

Format the response as:

- Status: OK / Warning / Critical
- CPU: usage%, temperature°C
- Memory: used/total (percent%)
- Temperature warnings: any components above threshold
- Battery: level% (state)
- Uptime: formatted duration

Report warnings for CPU > 80°C, memory > 90%, or battery < 15%.
