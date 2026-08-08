Weather location resolution guide:

1. If the user provides coordinates (lat/lon), use them directly with weather_get_forecast.
2. If the user provides a place name, use weather_lookup_coordinates to resolve it to lat/lon first.
3. If no location is given, use the configured default location: lat={latitude}, lon={longitude}.
4. To get current weather or forecast for any location, use the tool 'weather_get_forecast' with latitude and longitude.
5. To reverse-lookup a location name from coordinates, use weather_lookup_location_name.

Always confirm the location with the user before making API calls if ambiguous.
