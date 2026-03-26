#pragma once

#include <SDL3/SDL.h>
#include <string>

namespace husk::client::window {

class SdlApp {
public:
  SdlApp(const std::string &title, int width, int height);
  ~SdlApp();

  SdlApp(const SdlApp &) = delete;
  SdlApp &operator=(const SdlApp &) = delete;

  bool poll_events();

  SDL_Window *get_window() const { return m_window; }

  void get_drawable_size(int *width, int *height) const;

  bool is_resized() const { return m_resized; }
  void clear_resize_flag() { m_resized = false; }

private:
  SDL_Window *m_window{nullptr};
  bool m_resized{false};
};

} // namespace husk::client::window
