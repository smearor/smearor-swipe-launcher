Personalization guide:

Tools:

- get_current_location: Returns the user's current latitude, longitude, and location name
- get_timezone: Returns the current IANA timezone identifier (e.g. 'Europe/Berlin')
- get_locale: Returns the current system locale string (e.g. 'de-DE', 'en-US')
- get_personalization: Returns the full personalization profile as JSON
- set_current_location: Sets a runtime override for the user's location (latitude, longitude, optional location_name)
- set_locale: Sets a runtime override for the user's locale (e.g. 'de-DE', 'en-US')
- refresh_personalization: Clears all runtime overrides and re-queries system APIs

Resources:

- personalization://profile: Full personalization profile including location, timezone, locale, and unit/format preferences

Notes:

- Runtime overrides persist until refresh_personalization is called or the service restarts
- The profile includes temperature unit, wind speed unit, measurement system, date/time format, color scheme, and first day of week
