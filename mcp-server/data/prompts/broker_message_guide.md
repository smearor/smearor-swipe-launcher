The central message broker allows publishing JSON payloads to named topics. Use the 'send_message' tool with parameters:

- topic: the broker topic name (string)
- payload: a JSON object to publish
- target_instance_id: optional target widget/service instance ID Widgets and services subscribe to topics and react to incoming messages.
