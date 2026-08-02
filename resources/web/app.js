/* Smearor Swipe Launcher — Web Instance Client JavaScript */

(function () {
    "use strict";

    var wsSocket = null;

    function sendAction(instanceId, pluginId, action, payload) {
        if (wsSocket && wsSocket.readyState === WebSocket.OPEN) {
            var msg = {plugin_id: pluginId, action: action};
            if (payload) {
                msg.payload = payload;
            }
            wsSocket.send(JSON.stringify(msg));
            return;
        }
        // Fallback to HTTP POST when WebSocket is not available
        var body = payload ? {payload: payload} : {};
        fetch("/instances/" + encodeURIComponent(instanceId) + "/" + encodeURIComponent(pluginId) + "/" + encodeURIComponent(action), {
            method: "POST",
            headers: {"Content-Type": "application/json"},
            body: JSON.stringify(body)
        }).catch(function (err) {
            console.error("Action '" + action + "' failed:", err);
        });
    }

    function handleWebUpdate(update) {
        if (update.topic === "area.changed" && update.payload) {
            try {
                var data = typeof update.payload === "string" ? JSON.parse(update.payload) : update.payload;
                if (data.widgets_html) {
                    var container = document.querySelector(".launcher-areas");
                    if (container) {
                        container.innerHTML = data.widgets_html;
                        init();
                    }
                }
            } catch (err) {
                console.error("Failed to handle area.changed:", err);
            }
            return;
        }
        if (update.topic === "widget.update" && update.payload) {
            try {
                var data = typeof update.payload === "string" ? JSON.parse(update.payload) : update.payload;
                if (data.plugin_id && data.html) {
                    var old = document.querySelector('[data-plugin-id="' + data.plugin_id + '"]');
                    if (old) {
                        old.outerHTML = data.html;
                        init();
                    }
                }
            } catch (err) {
                console.error("Failed to handle widget.update:", err);
            }
            return;
        }
        var event = new CustomEvent("web-update", {detail: update});
        document.dispatchEvent(event);
    }

    function connectWebSocket(instanceId) {
        var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        var wsUrl = protocol + "//" + window.location.host + "/instances/" + encodeURIComponent(instanceId) + "/ws";

        var socket = new WebSocket(wsUrl);

        socket.onmessage = function (event) {
            try {
                var update = JSON.parse(event.data);
                handleWebUpdate(update);
            } catch (err) {
                console.error("Failed to parse WebSocket message:", err);
            }
        };

        socket.onopen = function () {
            wsSocket = socket;
        };

        socket.onclose = function () {
            wsSocket = null;
            console.log("WebSocket closed, reconnecting in 3s...");
            setTimeout(function () {
                connectWebSocket(instanceId);
            }, 3000);
        };

        socket.onerror = function (err) {
            console.error("WebSocket error:", err);
        };

        return socket;
    }

    function init() {
        var elements = document.querySelectorAll("[data-action-source]");
        elements.forEach(function (element) {
            var pluginId = element.dataset.pluginId;
            var instanceId = element.dataset.instanceId;

            if (!pluginId || !instanceId) return;

            var clickAction = element.dataset.clickAction || "click";
            var longpressAction = element.dataset.longpressAction || "longpress";

            element.addEventListener("click", function () {
                sendAction(instanceId, pluginId, clickAction);
            });

            element.addEventListener("contextmenu", function (event) {
                event.preventDefault();
                sendAction(instanceId, pluginId, longpressAction);
            });

            if (element.dataset.swipeActions === "true") {
                element.addEventListener("wheel", function (event) {
                    event.preventDefault();
                    if (event.deltaY < 0) {
                        sendAction(instanceId, pluginId, "scroll_up");
                    } else if (event.deltaY > 0) {
                        sendAction(instanceId, pluginId, "scroll_down");
                    }
                }, {passive: false});

                var touchStartY = 0;
                var touchStartX = 0;
                element.addEventListener("touchstart", function (event) {
                    if (event.touches.length === 1) {
                        touchStartY = event.touches[0].clientY;
                        touchStartX = event.touches[0].clientX;
                    }
                }, {passive: true});

                element.addEventListener("touchend", function (event) {
                    var touchEndY = event.changedTouches[0].clientY;
                    var touchEndX = event.changedTouches[0].clientX;
                    var deltaY = touchStartY - touchEndY;
                    var deltaX = touchStartX - touchEndX;
                    var threshold = 30;
                    if (Math.abs(deltaY) > Math.abs(deltaX) && Math.abs(deltaY) > threshold) {
                        if (deltaY > 0) {
                            sendAction(instanceId, pluginId, "swipe_up");
                        } else {
                            sendAction(instanceId, pluginId, "swipe_down");
                        }
                    }
                }, {passive: true});
            }
        });

        if (!wsSocket) {
            var instanceMeta = document.querySelector("meta[name='instance-id']");
            if (instanceMeta) {
                connectWebSocket(instanceMeta.content);
            }
        }
    }

    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", init);
    } else {
        init();
    }
})();
