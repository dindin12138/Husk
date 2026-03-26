#include "SdlApp.hpp"
#include <stdexcept>

namespace husk::client::window {

SdlApp::SdlApp(const std::string &title, int width, int height) {
  if (SDL_Init(SDL_INIT_VIDEO) < 0) {
    throw std::runtime_error(std::string("Failed to initialize SDL3: ") +
                             SDL_GetError());
  }

  m_window =
      SDL_CreateWindow(title.c_str(), width, height,
                       SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY);

  if (!m_window) {
    SDL_Quit();
    throw std::runtime_error(std::string("Failed to create SDL3 window: ") +
                             SDL_GetError());
  }
}

SdlApp::~SdlApp() {
  if (m_window) {
    SDL_DestroyWindow(m_window);
  }
  SDL_Quit();
}

bool SdlApp::poll_events() {
  SDL_Event event;
  while (SDL_PollEvent(&event)) {
    if (event.type == SDL_EVENT_QUIT) {
      return false;
    }

    if (event.type == SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED ||
        event.type == SDL_EVENT_WINDOW_RESIZED) {
      m_resized = true;
    }
  }
  return true;
}

void SdlApp::get_drawable_size(int *width, int *height) const {
  SDL_GetWindowSizeInPixels(m_window, width, height);
}

} // namespace husk::client::window
