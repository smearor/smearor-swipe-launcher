use smearor_model_macropad::MacroPadCommand;
use smearor_model_macropad::MacroPadCommandMessage;
use smearor_model_mcp::InvokeToolMessage;
use smearor_swipe_launcher_plugin_api::FfiEnvelope;
use smearor_swipe_launcher_plugin_api::FfiEnvelopePayload;
use smearor_swipe_launcher_plugin_api::MessageTopic;
use smearor_swipe_launcher_plugin_api::TypedMessage;
use smearor_swipe_launcher_plugin_api::box_payload;
use tracing::debug;
use tracing::trace;

use smearor_swipe_launcher_plugin_api::default_clone_payload;
use smearor_swipe_launcher_plugin_api::default_destroy_payload;

/// Extract a grid slice from an RGBA pixel buffer.
///
/// Used to split a 2D span group's combined render into individual button
/// images. Crops the region (x_offset, y_offset) to (slice_width, slice_height)
/// from the source buffer.
fn extract_grid_slice(pixels: &[u8], src_width: u32, src_height: u32, x_offset: u32, y_offset: u32, slice_width: u32, slice_height: u32) -> Vec<u8> {
    let _ = src_height;
    let mut result = Vec::with_capacity((slice_width * slice_height * 4) as usize);
    for y in y_offset..(y_offset + slice_height) {
        let start = ((y * src_width + x_offset) * 4) as usize;
        let end = start + (slice_width * 4) as usize;
        result.extend_from_slice(&pixels[start..end]);
    }
    result
}

impl super::LauncherHost {
    /// Check if a specific trigger type (e.g. "hold_topic", "double_press_topic")
    /// is configured for the plugin at `button_index` in the given instance's
    /// currently visible area.
    ///
    /// Uses the physical-to-logical button map built during rendering to
    /// account for 2D span group alignment shifts.
    pub(crate) fn is_trigger_configured(&self, instance_id: &str, button_index: u8, trigger_field: &str) -> bool {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let plugin_id = if let Ok(map) = instance.button_map.lock() {
                    map.as_ref().and_then(|m| m.get(button_index as usize).and_then(|id| id.clone()))
                } else {
                    return false;
                };
                let Some(plugin_id) = plugin_id else {
                    return false;
                };
                if let Some(config) = instance.config.get_plugin_config(&plugin_id) {
                    return config.get(trigger_field).and_then(|v| v.as_str()).is_some();
                }
            }
        }
        false
    }

    /// Get the span group name and all physical button indices in that group
    /// for the given physical button. Returns `None` if the button is not
    /// part of a span group.
    ///
    /// Uses the physical-to-logical button map built during rendering.
    pub(crate) fn get_span_group_for_button(&self, instance_id: &str, button_index: u8) -> Option<(String, Vec<u8>)> {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let button_map = if let Ok(map) = instance.button_map.lock() {
                    map.clone().unwrap_or_default()
                } else {
                    return None;
                };
                let plugin_id = button_map.get(button_index as usize).and_then(|id| id.as_ref().map(|s| s.as_str()))?;

                let entries = if let Ok(area_manager) = instance.area_manager.lock() {
                    area_manager.visible_area_plugin_entries()
                } else {
                    return None;
                };
                let target_entry = entries.iter().find(|e| e.id == plugin_id)?;
                let span_group = target_entry.span_group.clone()?;

                let group_plugin_ids: std::collections::HashSet<&str> = entries
                    .iter()
                    .filter(|e| e.span_group.as_ref() == Some(&span_group))
                    .map(|e| e.id.as_str())
                    .collect();

                let mut group_buttons: Vec<u8> = button_map
                    .iter()
                    .enumerate()
                    .filter(|(_, id)| id.as_ref().map_or(false, |pid| group_plugin_ids.contains(pid.as_str())))
                    .map(|(i, _)| i as u8)
                    .collect();
                group_buttons.sort();

                return Some((span_group, group_buttons));
            }
        }
        None
    }

    /// Dispatch a `InvokeToolMessage` with the given action to the plugin at
    /// `button_index` in the instance's currently visible area.
    ///
    /// Uses the physical-to-logical button map built during rendering.
    pub(crate) fn dispatch_macropad_action(&self, instance_id: &str, button_index: u8, action: &str) {
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                let plugin_id = if let Ok(map) = instance.button_map.lock() {
                    map.as_ref().and_then(|m| m.get(button_index as usize).and_then(|id| id.clone()))
                } else {
                    None
                };

                if let Some(plugin_id) = plugin_id {
                    let tool_name = format!("button_{}", plugin_id);
                    let correlation_id = format!("macropad-{}-{}", instance_id, button_index);
                    let arguments = format!(r#"{{"action":"{}"}}"#, action);
                    let invoke_msg = InvokeToolMessage::new(&tool_name, &correlation_id, &arguments);
                    let payload_ptr = box_payload(invoke_msg);
                    let invoke_envelope = FfiEnvelope::builder()
                        .sender_id(instance_id)
                        .target_instance_id("*")
                        .topic(InvokeToolMessage::topic())
                        .type_id(FfiEnvelopePayload::<InvokeToolMessage>::TYPE_ID)
                        .payload(payload_ptr)
                        .destroy_payload(Some(default_destroy_payload))
                        .clone_payload(Some(default_clone_payload::<InvokeToolMessage>))
                        .build();
                    instance.handle_message(invoke_envelope);
                    debug!("MacroPad: dispatched {} to plugin '{}' for instance '{}'", action, plugin_id, instance_id);
                } else {
                    debug!("MacroPad: no plugin at physical button {} for instance '{}'", button_index, instance_id);
                }
            } else {
                debug!("MacroPad: instance '{}' not found", instance_id);
            }
        }
    }

    /// Render all visible area plugins to button images and send them to the MacroPad device.
    ///
    /// For each plugin in the currently visible area, calls `render_graphic`
    /// with the device's key dimensions and sends a `SetButtonImage` command
    /// to the MacroPad service via the message broker.
    pub fn render_buttons_to_device(&self, instance_id: &str) {
        let (device_id, driver, key_count, key_columns, key_width, key_height) = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(metadata_guard) = instance.device_metadata.lock() else {
                return;
            };
            let Some(ref metadata) = *metadata_guard else {
                return;
            };
            (
                metadata.device_id.clone(),
                metadata.driver.clone(),
                metadata.key_count,
                metadata.key_columns,
                metadata.key_width,
                metadata.key_height,
            )
        };

        let plugin_entries: Vec<smearor_model_plugin::PluginEntry> = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(area_manager) = instance.area_manager.lock() else {
                return;
            };
            area_manager.visible_area_plugin_entries()
        };

        debug!(
            "Rendering {} buttons to device '{}' ({}x{}) for instance '{}'",
            plugin_entries.len(),
            device_id,
            key_width,
            key_height,
            instance_id
        );

        // Track which physical buttons are rendered, for gap clearing and button_map.
        let mut rendered_buttons: Vec<bool> = vec![false; key_count as usize];
        let mut button_map: Vec<Option<String>> = vec![None; key_count as usize];

        // Group plugins by span_group. Plugins without span_group are individual.
        // We iterate in order, collecting consecutive plugins with the same span_group.
        let mut button_index: usize = 0;
        let mut iter = plugin_entries.iter().enumerate().peekable();

        while let Some((_, entry)) = iter.next() {
            if button_index as u8 >= key_count {
                break;
            }

            if let Some(ref span_group) = entry.span_group {
                // Collect all consecutive plugins with the same span_group.
                let mut group_members = vec![entry];
                while let Some(&(_, peek_entry)) = iter.peek() {
                    if peek_entry.span_group.as_ref() == Some(span_group) {
                        group_members.push(peek_entry);
                        iter.next();
                    } else {
                        break;
                    }
                }

                // Sort by span_index for deterministic ordering.
                group_members.sort_by_key(|e| e.span_index.unwrap_or(0));

                // Read span_rows and span_cols from the first member.
                // If both absent: backward-compatible 1×N horizontal span.
                let member_count = group_members.len() as u32;
                let (span_rows, span_cols) = match (group_members[0].span_rows, group_members[0].span_cols) {
                    (Some(rows), Some(cols)) => {
                        let expected = rows * cols;
                        if expected != member_count {
                            debug!(
                                "Span group '{}': member count {} does not match span_rows*span_cols={} ({}×{}), falling back to 1×N",
                                span_group, member_count, expected, rows, cols
                            );
                            (1, member_count)
                        } else {
                            (rows, cols)
                        }
                    }
                    _ => (1, member_count),
                };
                let group_size = span_rows * span_cols;

                // Find the next available position where the entire span_rows × span_cols
                // rectangle fits without overlapping any already-rendered button.
                let device_rows = if key_columns > 0 { key_count / key_columns } else { 0 };
                let mut effective_base: Option<u32> = None;
                let start_button = button_index as u32;

                'search: for candidate in start_button..(key_count as u32) {
                    let cand_col = candidate % key_columns as u32;
                    let cand_row = candidate / key_columns as u32;

                    // Check column overflow.
                    if cand_col + span_cols > key_columns as u32 {
                        continue;
                    }

                    // Check row overflow.
                    if device_rows > 0 && cand_row + span_rows > device_rows as u32 {
                        break;
                    }

                    // Check all buttons in the rectangle are free.
                    for r in 0..span_rows {
                        for c in 0..span_cols {
                            let physical = candidate + r * key_columns as u32 + c;
                            if physical as u8 >= key_count || rendered_buttons[physical as usize] {
                                continue 'search;
                            }
                        }
                    }

                    effective_base = Some(candidate);
                    break 'search;
                }

                let Some(effective_base) = effective_base else {
                    debug!("Span group '{}': no free position for {}×{} grid, skipping", span_group, span_rows, span_cols);
                    continue;
                };

                debug!(
                    "Span group '{}': placed at button {} (col={}, row={})",
                    span_group,
                    effective_base,
                    effective_base % key_columns as u32,
                    effective_base / key_columns as u32
                );

                let combined_width = key_width * span_cols;
                let combined_height = key_height * span_rows;

                // Render the first member at combined dimensions.
                let first_plugin_id = &group_members[0].id;
                let namespaced_id = format!("{}:{}", instance_id, first_plugin_id);
                let graphic = {
                    let Ok(instances) = self.instances.lock() else {
                        continue;
                    };
                    let Some(instance) = instances.get(instance_id) else {
                        continue;
                    };
                    let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                        debug!("Span group: plugin '{}' not found for rendering, skipping", namespaced_id);
                        continue;
                    };
                    unsafe { plugin.render_graphic(combined_width, combined_height) }
                };

                if let Some(graphic) = graphic {
                    let pixels = graphic.as_pixels();
                    let graphic_width = graphic.width;
                    let graphic_height = graphic.height;

                    // Split the combined image into grid slices and send each to its physical button.
                    for (i, member) in group_members.iter().enumerate() {
                        let row = i as u32 / span_cols;
                        let col = i as u32 % span_cols;
                        let physical_button = effective_base + row * key_columns as u32 + col;
                        if physical_button as u8 >= key_count {
                            break;
                        }
                        let x_offset = col * key_width;
                        let y_offset = row * key_height;
                        let slice_pixels = extract_grid_slice(pixels, graphic_width, graphic_height, x_offset, y_offset, key_width, key_height);
                        self.send_button_image(&device_id, &driver, instance_id, physical_button as u8, key_width, key_height, slice_pixels);
                        rendered_buttons[physical_button as usize] = true;
                        button_map[physical_button as usize] = Some(member.id.clone());
                        debug!("Sent span group slice {} (plugin '{}') to button {} on device '{}'", i, member.id, physical_button, device_id);
                    }
                } else {
                    debug!("Span group: plugin '{}' has no render_graphic, skipping {} buttons", first_plugin_id, group_size);
                }
                button_index = (effective_base + span_cols) as usize;
            } else {
                // Individual plugin — render at standard dimensions.
                // Search from the beginning for the first free button to fill gaps
                // left by span groups that were placed further ahead.
                button_index = 0;
                while button_index < key_count as usize && rendered_buttons[button_index] {
                    button_index += 1;
                }
                if button_index as u8 >= key_count {
                    break;
                }

                let plugin_id = &entry.id;
                let namespaced_id = format!("{}:{}", instance_id, plugin_id);
                let graphic = {
                    let Ok(instances) = self.instances.lock() else {
                        continue;
                    };
                    let Some(instance) = instances.get(instance_id) else {
                        continue;
                    };
                    let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                        debug!("Plugin '{}' not found for rendering, skipping", namespaced_id);
                        button_index += 1;
                        continue;
                    };
                    unsafe { plugin.render_graphic(key_width, key_height) }
                };

                if let Some(graphic) = graphic {
                    let pixels = graphic.as_pixels().to_vec();
                    self.send_button_image(&device_id, &driver, instance_id, button_index as u8, graphic.width, graphic.height, pixels);
                    rendered_buttons[button_index] = true;
                    button_map[button_index] = Some(plugin_id.clone());
                    trace!("Sent button image for index {} (plugin '{}') to device '{}'", button_index, plugin_id, device_id);
                } else {
                    debug!("Plugin '{}' has no render_graphic, skipping button {}", plugin_id, button_index);
                }
                button_index += 1;
            }
        }

        // Clear all buttons that were not rendered (gaps from alignment shifts + trailing empty buttons).
        for idx in 0..key_count as usize {
            if !rendered_buttons[idx] {
                let command = MacroPadCommand::clear_button(idx as u8);
                let msg = MacroPadCommandMessage::new(&device_id, command);
                let payload_ptr = box_payload(msg);
                let envelope = FfiEnvelope::builder()
                    .sender_id(instance_id)
                    .target_instance_id(driver.as_str())
                    .topic(MacroPadCommandMessage::topic())
                    .type_id(FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID)
                    .payload(payload_ptr)
                    .destroy_payload(Some(default_destroy_payload))
                    .clone_payload(Some(default_clone_payload::<MacroPadCommandMessage>))
                    .build();
                let _ = self.broker_sender.send(envelope);
                trace!("Cleared unrendered button {} on device '{}'", idx, device_id);
            }
        }

        // Store the button map for input dispatch.
        if let Ok(instances) = self.instances.lock() {
            if let Some(instance) = instances.get(instance_id) {
                if let Ok(mut map) = instance.button_map.lock() {
                    *map = Some(button_map);
                }
            }
        }
    }

    /// Send a `SetButtonImage` command for a single button to the MacroPad device.
    pub(crate) fn send_button_image(&self, device_id: &str, driver: &str, instance_id: &str, button_index: u8, width: u32, height: u32, pixels: Vec<u8>) {
        let command = MacroPadCommand::set_button_image(button_index, width, height, pixels);
        let msg = MacroPadCommandMessage::new(device_id, command);
        let payload_ptr = box_payload(msg);
        let envelope = FfiEnvelope::builder()
            .sender_id(instance_id)
            .target_instance_id(driver)
            .topic(MacroPadCommandMessage::topic())
            .type_id(FfiEnvelopePayload::<MacroPadCommandMessage>::TYPE_ID)
            .payload(payload_ptr)
            .destroy_payload(Some(default_destroy_payload))
            .clone_payload(Some(default_clone_payload::<MacroPadCommandMessage>))
            .build();
        let _ = self.broker_sender.send(envelope);
    }

    /// Re-render a single plugin's button image and send it to the MacroPad device.
    ///
    /// Called when a widget sends a `widget.update` message indicating its
    /// visual state has changed. Finds the plugin's button index in the
    /// visible area and sends only that button's updated image.
    /// If the plugin is part of a span group, re-renders the entire group.
    pub fn render_single_button_to_device(&self, instance_id: &str, plugin_id: &str) {
        let (device_id, driver, key_count, key_columns, key_width, key_height) = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(metadata_guard) = instance.device_metadata.lock() else {
                return;
            };
            let Some(ref metadata) = *metadata_guard else {
                return;
            };
            (
                metadata.device_id.clone(),
                metadata.driver.clone(),
                metadata.key_count,
                metadata.key_columns,
                metadata.key_width,
                metadata.key_height,
            )
        };

        let plugin_entries: Vec<smearor_model_plugin::PluginEntry> = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(area_manager) = instance.area_manager.lock() else {
                return;
            };
            area_manager.visible_area_plugin_entries()
        };

        // Find the plugin and check if it's part of a span group.
        let target_entry = plugin_entries.iter().find(|e| e.id == plugin_id);
        let Some(target_entry) = target_entry else {
            trace!("render_single_button: plugin '{}' not in visible area for instance '{}'", plugin_id, instance_id);
            return;
        };

        if let Some(ref span_group) = target_entry.span_group {
            // Plugin is part of a span group — re-render the entire group.
            let mut group_members: Vec<&smearor_model_plugin::PluginEntry> =
                plugin_entries.iter().filter(|e| e.span_group.as_ref() == Some(span_group)).collect();
            group_members.sort_by_key(|e| e.span_index.unwrap_or(0));

            // Read span_rows and span_cols from the first member.
            let member_count = group_members.len() as u32;
            let (span_rows, span_cols) = match (group_members[0].span_rows, group_members[0].span_cols) {
                (Some(rows), Some(cols)) => {
                    let expected = rows * cols;
                    if expected != member_count { (1, member_count) } else { (rows, cols) }
                }
                _ => (1, member_count),
            };
            let group_size = span_rows * span_cols;

            let combined_width = key_width * span_cols;
            let combined_height = key_height * span_rows;

            // Find the physical starting button index for this group from the button_map.
            let first_member_id = &group_members[0].id;
            let button_index = {
                let Ok(instances) = self.instances.lock() else {
                    return;
                };
                let Some(instance) = instances.get(instance_id) else {
                    return;
                };
                let Ok(map) = instance.button_map.lock() else {
                    return;
                };
                let Some(ref button_map) = *map else {
                    return;
                };
                match button_map.iter().position(|id| id.as_deref() == Some(first_member_id)) {
                    Some(idx) => idx,
                    None => return,
                }
            };

            if button_index as u8 >= key_count {
                return;
            }

            // Alignment validation: check row and column overflow.
            let base_button = button_index as u32;
            let base_col = base_button % key_columns as u32;
            let base_row = base_button / key_columns as u32;

            let mut effective_base = base_button;
            if base_col + span_cols > key_columns as u32 {
                let next_row_start = (base_row + 1) * key_columns as u32;
                debug!(
                    "render_single_button: span group '{}' would overflow row boundary, advancing to button {}",
                    span_group, next_row_start
                );
                effective_base = next_row_start;
            }

            let device_rows = if key_columns > 0 { key_count / key_columns } else { 0 };
            let effective_row = effective_base / key_columns as u32;
            if device_rows > 0 && effective_row + span_rows > device_rows as u32 {
                debug!("render_single_button: span group '{}' would overflow device bottom, skipping", span_group);
                return;
            }

            let namespaced_id = format!("{}:{}", instance_id, first_member_id);
            let graphic = {
                let Ok(instances) = self.instances.lock() else {
                    return;
                };
                let Some(instance) = instances.get(instance_id) else {
                    return;
                };
                let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                    trace!("render_single_button: span group plugin '{}' not found, skipping", namespaced_id);
                    return;
                };
                unsafe { plugin.render_graphic(combined_width, combined_height) }
            };

            if let Some(graphic) = graphic {
                let pixels = graphic.as_pixels();
                let graphic_width = graphic.width;
                let graphic_height = graphic.height;

                for (i, member) in group_members.iter().enumerate() {
                    let row = i as u32 / span_cols;
                    let col = i as u32 % span_cols;
                    let physical_button = effective_base + row * key_columns as u32 + col;
                    if physical_button as u8 >= key_count {
                        break;
                    }
                    let x_offset = col * key_width;
                    let y_offset = row * key_height;
                    let slice_pixels = extract_grid_slice(pixels, graphic_width, graphic_height, x_offset, y_offset, key_width, key_height);
                    self.send_button_image(&device_id, &driver, instance_id, physical_button as u8, key_width, key_height, slice_pixels);
                    trace!(
                        "Re-rendered span group slice {} (plugin '{}') for button {} on device '{}'",
                        i, member.id, physical_button, device_id
                    );
                }
            } else {
                trace!("render_single_button: span group plugin '{}' has no render_graphic, skipping", first_member_id);
            }
            return;
        }

        // Individual plugin — render at standard dimensions.
        // Look up physical button index from the button_map.
        let button_index = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Ok(map) = instance.button_map.lock() else {
                return;
            };
            let Some(ref button_map) = *map else {
                return;
            };
            match button_map.iter().position(|id| id.as_deref() == Some(plugin_id)) {
                Some(idx) => idx,
                None => {
                    trace!("render_single_button: plugin '{}' not in button_map for instance '{}'", plugin_id, instance_id);
                    return;
                }
            }
        };

        if button_index as u8 >= key_count {
            trace!("render_single_button: button index {} >= key_count {} for plugin '{}'", button_index, key_count, plugin_id);
            return;
        }

        let namespaced_id = format!("{}:{}", instance_id, plugin_id);
        let graphic = {
            let Ok(instances) = self.instances.lock() else {
                return;
            };
            let Some(instance) = instances.get(instance_id) else {
                return;
            };
            let Some(plugin) = instance.plugin_manager.plugins.get(&namespaced_id) else {
                trace!("render_single_button: plugin '{}' not found, skipping", namespaced_id);
                return;
            };
            unsafe { plugin.render_graphic(key_width, key_height) }
        };

        if let Some(graphic) = graphic {
            let pixels = graphic.as_pixels().to_vec();
            self.send_button_image(&device_id, &driver, instance_id, button_index as u8, graphic.width, graphic.height, pixels);
            trace!("Re-rendered single button {} (plugin '{}') for device '{}'", button_index, plugin_id, device_id);
        } else {
            trace!("render_single_button: plugin '{}' has no render_graphic, skipping", plugin_id);
        }
    }
}
