# Native website controls

The Servo embedder now presents active-tab website alerts, prompts, confirmation dialogs, select dropdowns, color pickers, context menus and user-confirmed file selection. Native browser chrome identifies the current top-level origin; an embedded frame may have a different origin. Only an explicit file selection is submitted. Website permission requests remain denied, including microphone and camera. Requests from background tabs are ignored; popup requests remain denied unless the user explicitly chooses to open a link from the context menu.

The F11 borderless window and Servo fullscreen notifications are integrated. The browser still does not promise support for arbitrary popups, IME, downloads, signed updates, sandboxing, or all web standards. Test with non-sensitive pages. File selection, media, context-menu actions, and DOM dialogs require runtime validation.
