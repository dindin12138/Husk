#pragma once

#include "client/window/SdlApp.hpp"
#include <webgpu/webgpu.h>
#include <webgpu/wgpu.h>

namespace husk::client::render {

class WgpuRenderer {
public:
  WgpuRenderer(window::SdlApp &app);
  ~WgpuRenderer();

  void resize(int width, int height);
  void draw_frame();

private:
  void init_webgpu(window::SdlApp &app);
  void configure_surface(int width, int height);

private:
  WGPUInstance m_instance{nullptr};
  WGPUDevice m_device{nullptr};
  WGPUQueue m_queue{nullptr};
  WGPUSurface m_surface{nullptr};

  WGPUTextureFormat m_swapchain_format;
  WGPUCompositeAlphaMode m_alpha_mode;
};

} // namespace husk::client::render
