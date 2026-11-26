# Desktop feature update

This update adds session-local appearance (system/light/dark), search provider selection, validated home shortcuts, configurable tab limit, tab pin/reorder/duplicate, loading indicators, history filtering and clearing, and borderless fullscreen (F11 or Escape to leave). The window title is only updated when it changes. The previous wheel scaling and temporary profile restrictions are retained.

Pins prevent accidental closing. Tab duplication and restore load the URL, not page state. Home shortcuts and appearance are intentionally session-only; no normal persistent profile exists yet. A loading indicator is not a percentage progress bar. Fullscreen is a window feature, not yet a verified video fullscreen integration.

Regression tests include URL validation, search encoding, shortcut bounds and tab reorder/pin invariants. The native Windows workflow still needs to pass and manual website/input checks are required. Downloads, web permission UI, persistent profiles, filter lists, password manager, extensions, verified fingerprinting protection, process isolation, signed updates and installer are not implemented by this revision. Do not use the preview for sensitive browsing.
