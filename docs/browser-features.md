# Lutra browser usability update

The native new-tab page includes a search field and shortcuts. Navigation remains HTTPS-only. Wheel line events use Servo 0.7's 76-unit conversion; touchpad pixel events remain unchanged. Scroll speed is adjustable for wheel line events.

Session history, bookmarks, and recently closed URLs are bounded and memory-only. Reopening a tab restores its URL, not form data or back history. Settings are session-only. Home opens a new tab rather than discarding a page.

Shortcuts: Ctrl+T/W/L/R/D, Ctrl+Shift+T, Ctrl+Tab/Shift+Tab, Ctrl+Plus/Minus/0, F5, Alt+Left/Right/Home. Page zoom uses Servo's native API.

Manual Windows regression checklist: start without URL; enter a query on the home page; navigate to a test page; test wheel, touchpad, link clicks, text entry and address focus; switch and close tabs; zoom/reset; bookmark and reopen; clear history; resize at 100%, 150% and 200% display scaling; close the browser. Verify no persistent history after restarting. The automated startup check is not a substitute for these tests.

Not implemented or verified: downloads, persistent optional profiles, extensions, robust popup handling, media compatibility, site isolation, password management, accessibility, and comprehensive security testing. Do not use this preview for sensitive browsing.
