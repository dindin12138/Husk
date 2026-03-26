#include "client/render/WgpuRenderer.hpp"
#include "client/window/SdlApp.hpp"
#include <iostream>

using namespace husk::client;

int main(int argc, char *argv[]) {
  std::cout << "[Husk] Booting The Canvas..." << std::endl;

  try {
    window::SdlApp app("Husk Terminal", 1024, 768);

    render::WgpuRenderer renderer(app);

    bool running = true;
    while (running) {
      running = app.poll_events();

      if (app.is_resized()) {
        int width, height;
        app.get_drawable_size(&width, &height);
        renderer.resize(width, height);
        app.clear_resize_flag();
        std::cout << "[Husk] Resized to " << width << "x" << height
                  << std::endl;
      }

      renderer.draw_frame();
    }

  } catch (const std::exception &e) {
    std::cerr << "[Husk Fatal Error] " << e.what() << std::endl;
    return 1;
  }

  std::cout << "[Husk] Shutting down gracefully." << std::endl;
  return 0;
}
