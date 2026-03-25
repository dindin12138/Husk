#include "Terminal.hpp"

namespace husk::vt {

Terminal::Terminal(uint16_t cols, uint16_t rows, size_t max_scrollback) {
  GhosttyTerminalOptions opts = {
      .cols = cols, .rows = rows, .max_scrollback = max_scrollback};

  if (ghostty_terminal_new(nullptr, &m_handle, opts) != GHOSTTY_SUCCESS) {
    throw std::runtime_error("Failed to allocate GhosttyTerminal");
  }
}

Terminal::~Terminal() {
  if (m_handle) {
    ghostty_terminal_free(m_handle);
  }
}

Terminal::Terminal(Terminal &&other) noexcept : m_handle(other.m_handle) {
  other.m_handle = nullptr;
}

Terminal &Terminal::operator=(Terminal &&other) noexcept {
  if (this != &other) {
    if (m_handle) {
      ghostty_terminal_free(m_handle);
    }
    m_handle = other.m_handle;
    other.m_handle = nullptr;
  }
  return *this;
}

void Terminal::write(std::string_view data) {
  ghostty_terminal_vt_write(
      m_handle, reinterpret_cast<const uint8_t *>(data.data()), data.size());
}

void Terminal::resize(uint16_t cols, uint16_t rows) {
  ghostty_terminal_resize(m_handle, cols, rows);
}

void Terminal::reset() { ghostty_terminal_reset(m_handle); }

} // namespace husk::vt
