Wallpaper management guide:

Tools:

- add_wallpaper_theme: Permanently append a new wallpaper theme to the configuration store
- remove_wallpaper_theme: Delete a wallpaper theme from the configuration store
- select_wallpaper_theme: Select a wallpaper theme by name without starting it
- start_selected_wallpaper_process: Start the currently selected theme (stops any running theme first)
- stop_current_wallpaper_process: Stop the currently running wallpaper process immediately

Resources:

- wallpaper://status: Current wallpaper service status including running theme and configured themes
- wallpaper://themes: List of all configured wallpaper themes with their configurations

Notes:

- Theme types: Video, Image, Application
- Use select_wallpaper_theme then start_selected_wallpaper_process to switch themes
- stop_current_wallpaper_process should be called before exiting or switching
