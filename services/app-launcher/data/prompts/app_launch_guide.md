App Launch Guide

To launch an application:

1. Use 'app_launcher_search_apps' with a query to find the desktop file path
2. Use 'app_launcher_exec' with the desktop_file path to launch it
3. Use 'app_launcher_terminate' to stop a running application

Tools:

- app_launcher_search_apps: Search available apps by name
- app_launcher_exec: Launch an app by desktop file path
- app_launcher_terminate: Terminate a running app

Resources:

- app_launcher://running_apps: List running tracked apps
- app_launcher://available_apps: List all available .desktop files (supports pagination)
