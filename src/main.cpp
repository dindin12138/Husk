#include <cstring>
#include <iostream>
#include <stdexcept>

extern "C" {
#include <ghostty/vt.h>
}

int main() {
  std::cout << "Starting Husk (Powered by libghostty)...\n";

  GhosttyTerminalOptions opts = {
      .cols = 80, .rows = 24, .max_scrollback = 1000};

  GhosttyTerminal terminal = nullptr;
  if (ghostty_terminal_new(nullptr, &terminal, opts) != GHOSTTY_SUCCESS) {
    throw std::runtime_error("Failed to initialize libghostty terminal.");
  }

  const char *hello_seq = "\x1b[31;1mHello from Husk Shell!\x1b[0m\r\n";
  ghostty_terminal_vt_write(terminal, (const uint8_t *)hello_seq,
                            std::strlen(hello_seq));

  GhosttyRenderState render_state = nullptr;
  if (ghostty_render_state_new(nullptr, &render_state) != GHOSTTY_SUCCESS) {
    ghostty_terminal_free(terminal);
    throw std::runtime_error("Failed to initialize render state.");
  }

  GhosttyResult res_update =
      ghostty_render_state_update(render_state, terminal);
  if (res_update != GHOSTTY_SUCCESS) {
    std::cerr << "Failed to update render state.\n";
  }

  std::cout << "Terminal state updated successfully!\n";

  ghostty_render_state_free(render_state);
  ghostty_terminal_free(terminal);

  return 0;
}
