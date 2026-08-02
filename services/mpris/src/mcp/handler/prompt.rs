use crate::service::MprisService;
use smearor_model_mcp::InvokePromptError;
use smearor_model_mcp::InvokePromptMessage;
use smearor_model_mcp::InvokePromptResponse;
use smearor_model_mcp::PromptMessage;
use smearor_mpris_model::MprisMcpPrompts;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageBroadcaster;
use smearor_swipe_launcher_plugin_api::MessageHandler;
use std::str::FromStr;
use tracing::debug;

impl MessageHandler<FfiEnvelopePayload<InvokePromptMessage>> for MprisService {
    fn handle_message(&self, message: FfiEnvelopePayload<InvokePromptMessage>, _sender_id: &str) {
        let prompt_name = message.0.name.to_string();
        let correlation_id = message.0.correlation_id.to_string();
        debug!("MPRIS Service: InvokePromptMessage name={}", prompt_name);
        let broadcaster = self.get_broadcaster();
        let prompt = match MprisMcpPrompts::from_str(&prompt_name) {
            Ok(prompt) => prompt,
            Err(e) => {
                broadcaster.broadcast_message_to_topic(InvokePromptResponse::from(InvokePromptError::new(e, &correlation_id)));
                return;
            }
        };

        let response = match prompt {
            MprisMcpPrompts::MprisControlGuide => {
                let status = self.status_snapshot();
                let players_info = match &status {
                    Some(s) => {
                        let players: Vec<String> = s
                            .players
                            .iter()
                            .map(|p| format!("{} ({})", p.name.to_string(), p.bus_name.to_string()))
                            .collect();
                        if players.is_empty() {
                            "No MPRIS players currently available.".to_string()
                        } else {
                            format!("Available players: {}", players.join(", "))
                        }
                    }
                    None => "MPRIS status not yet available.".to_string(),
                };

                let content = format!(
                    "{players_info}\n\n\
                     You can control media playback using the following tools:\n\
                     - mpris_play: Start/resume playback (German: starten, abspielen, wiedergeben, Musik starten, Lied starten, Song starten, Wiedergabe starten)\n\
                     - mpris_pause: Pause playback (German: pausieren, Pause)\n\
                     - mpris_toggle_play_pause: Toggle play/pause (German: Play-Pause umschalten)\n\
                     - mpris_stop: Stop playback (German: stoppen, anhalten, Wiedergabe stoppen)\n\
                     - mpris_next_track: Skip to next track (German: nächstes Lied, nächster Track, weiter)\n\
                     - mpris_previous_track: Go to previous track (German: vorheriges Lied, vorheriger Track, zurück)\n\
                     - mpris_seek: Seek by offset in microseconds (German: vorspulen, zurückspulen)\n\
                     - mpris_set_position: Set absolute position in microseconds (German: Position setzen)\n\
                     - mpris_cycle_loop: Cycle loop mode (None -> Track -> Playlist) (German: Wiederholung, Loop)\n\
                     - mpris_toggle_shuffle: Toggle shuffle (German: Zufallswiedergabe, mischen)\n\
                     - mpris_next_player: Switch to next player (German: nächster Player)\n\
                     - mpris_previous_player: Switch to previous player (German: vorheriger Player)\n\
                     - mpris_raise: Bring player window to foreground (German: Player in den Vordergrund)\n\
                     - mpris_quit: Quit the player application (German: Player beenden, schließen)\n\
                     - mpris_refresh_status: Force status refresh (German: Status aktualisieren)\n\n\
                     Resources: mpris://status, mpris://players, mpris://playback, mpris://metadata"
                );

                let messages = vec![PromptMessage::new("system", &content)];
                InvokePromptResponse::success(&correlation_id, messages)
            }
        };
        broadcaster.broadcast_message_to_topic(response);
    }
}
