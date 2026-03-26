#pragma once

#include <cstdint>
#include <stdexcept>
#include <string_view>

extern "C" {
#include <ghostty/vt/terminal.h>
}

namespace husk::vt {

class Terminal {
public:
  Terminal(uint16_t cols, uint16_t rows, size_t max_scrollback = 1000);
  ~Terminal();

  Terminal(const Terminal &) = delete;
  Terminal &operator=(const Terminal &) = delete;

  Terminal(Terminal &&other) noexcept;
  Terminal &operator=(Terminal &&other) noexcept;

  void write(std::string_view data);
  void resize(uint16_t cols, uint16_t rows, uint32_t width_px = 0,
              uint32_t height_px = 0);
  void reset();

  GhosttyTerminal get_handle() const { return m_handle; }

private:
  GhosttyTerminal m_handle{nullptr};
};

} // namespace husk::vt
