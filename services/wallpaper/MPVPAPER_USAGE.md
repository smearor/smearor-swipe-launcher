Usage: mpvpaper [options] <output> <url|path filename>

Example: mpvpaper -vs -o "no-audio loop" DP-2 /path/to/video

Options:
--help -h Displays this help message
--help-output -d Displays all available outputs and quits
--verbose -v Be more verbose (-vv for higher verbosity)
--fork -f Forks mpvpaper so you can close the terminal
--auto-pause -p Automagically* pause mpv when the wallpaper is hidden
This saves CPU usage, more or less, seamlessly
--auto-stop -s Automagically* stop mpv when the wallpaper is hidden
This saves CPU/RAM usage, although more abruptly
--slideshow -n SECS Slideshow mode plays the next video in a playlist every ? seconds
And passes mpv options "loop loop-playlist" for convenience
--layer -l LAYER Specifies shell surface layer to run on (background by default)
--mpv-options -o "OPTIONS"    Forwards mpv options (Must be within quotes"")

* The auto options might not work as intended
  See the man page for more details
