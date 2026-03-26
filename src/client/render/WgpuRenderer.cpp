#include "WgpuRenderer.hpp"
#include <SDL3/SDL.h>
#include <iostream>
#include <stdexcept>
#include <string_view>

namespace husk::client::render {

static void onAdapterRequestEnded(WGPURequestAdapterStatus status,
                                  WGPUAdapter adapter, WGPUStringView message,
                                  void *userdata1, void *userdata2) {
  if (status == WGPURequestAdapterStatus_Success) {
    *static_cast<WGPUAdapter *>(userdata1) = adapter;
  } else {
    std::string_view msg(message.data ? message.data : "Unknown",
                         message.length);
    std::cerr << "[WebGPU] Could not get adapter: " << msg << std::endl;
  }
}

static void onDeviceRequestEnded(WGPURequestDeviceStatus status,
                                 WGPUDevice device, WGPUStringView message,
                                 void *userdata1, void *userdata2) {
  if (status == WGPURequestDeviceStatus_Success) {
    *static_cast<WGPUDevice *>(userdata1) = device;
  } else {
    std::string_view msg(message.data ? message.data : "Unknown",
                         message.length);
    std::cerr << "[WebGPU] Could not get device: " << msg << std::endl;
  }
}

WgpuRenderer::WgpuRenderer(window::SdlApp &app) {
  init_webgpu(app);

  int width, height;
  app.get_drawable_size(&width, &height);
  configure_surface(width, height);
}

WgpuRenderer::~WgpuRenderer() {
  if (m_surface)
    wgpuSurfaceRelease(m_surface);
  if (m_queue)
    wgpuQueueRelease(m_queue);
  if (m_device)
    wgpuDeviceRelease(m_device);
  if (m_instance)
    wgpuInstanceRelease(m_instance);
}

void WgpuRenderer::init_webgpu(window::SdlApp &app) {
  WGPUInstanceDescriptor instDesc = {};
  m_instance = wgpuCreateInstance(&instDesc);
  if (!m_instance)
    throw std::runtime_error("Failed to create WebGPU instance!");

  SDL_PropertiesID props = SDL_GetWindowProperties(app.get_window());

  void *wl_display = SDL_GetPointerProperty(
      props, SDL_PROP_WINDOW_WAYLAND_DISPLAY_POINTER, nullptr);
  void *wl_surface = SDL_GetPointerProperty(
      props, SDL_PROP_WINDOW_WAYLAND_SURFACE_POINTER, nullptr);
  void *x11_display = SDL_GetPointerProperty(
      props, SDL_PROP_WINDOW_X11_DISPLAY_POINTER, nullptr);
  int64_t x11_window =
      SDL_GetNumberProperty(props, SDL_PROP_WINDOW_X11_WINDOW_NUMBER, 0);

  if (wl_display && wl_surface) {
    std::cout << "[Husk GPU] Wayland Surface detected." << std::endl;
    WGPUSurfaceSourceWaylandSurface wlDesc = {};
    wlDesc.chain.sType = WGPUSType_SurfaceSourceWaylandSurface;
    wlDesc.display = wl_display;
    wlDesc.surface = wl_surface;

    WGPUSurfaceDescriptor surfDesc = {};
    surfDesc.nextInChain = reinterpret_cast<const WGPUChainedStruct *>(&wlDesc);
    m_surface = wgpuInstanceCreateSurface(m_instance, &surfDesc);
  } else if (x11_display && x11_window) {
    std::cout << "[Husk GPU] X11 Surface detected." << std::endl;
    WGPUSurfaceSourceXlibWindow x11Desc = {};
    x11Desc.chain.sType = WGPUSType_SurfaceSourceXlibWindow;
    x11Desc.display = x11_display;
    x11Desc.window = static_cast<uint32_t>(x11_window);

    WGPUSurfaceDescriptor surfDesc = {};
    surfDesc.nextInChain =
        reinterpret_cast<const WGPUChainedStruct *>(&x11Desc);
    m_surface = wgpuInstanceCreateSurface(m_instance, &surfDesc);
  } else {
    throw std::runtime_error("Unsupported display server for WebGPU surface!");
  }

  WGPURequestAdapterOptions adapterOpts = {};
  adapterOpts.compatibleSurface = m_surface;
  adapterOpts.powerPreference = WGPUPowerPreference_HighPerformance;

  WGPUAdapter adapter = nullptr;
  WGPURequestAdapterCallbackInfo adapterCbInfo = {};
  adapterCbInfo.callback = onAdapterRequestEnded;
  adapterCbInfo.userdata1 = &adapter;
  wgpuInstanceRequestAdapter(m_instance, &adapterOpts, adapterCbInfo);

  if (!adapter)
    throw std::runtime_error("Failed to request WebGPU adapter!");

  WGPUDeviceDescriptor deviceDesc = {};
  WGPURequestDeviceCallbackInfo deviceCbInfo = {};
  deviceCbInfo.callback = onDeviceRequestEnded;
  deviceCbInfo.userdata1 = &m_device;
  wgpuAdapterRequestDevice(adapter, &deviceDesc, deviceCbInfo);

  if (!m_device)
    throw std::runtime_error("Failed to request WebGPU device!");

  m_queue = wgpuDeviceGetQueue(m_device);

  WGPUSurfaceCapabilities caps = {};
  caps.nextInChain = nullptr;
  wgpuSurfaceGetCapabilities(m_surface, adapter, &caps);

  if (caps.formatCount > 0) {
    m_swapchain_format = caps.formats[0];
    std::cout << "[Husk GPU] Picked Surface Format: " << m_swapchain_format
              << std::endl;
  } else {
    m_swapchain_format = WGPUTextureFormat_BGRA8Unorm;
  }

  if (caps.alphaModeCount > 0) {
    m_alpha_mode = caps.alphaModes[0];
    std::cout << "[Husk GPU] Picked Alpha Mode: " << m_alpha_mode << std::endl;
  } else {
    m_alpha_mode = WGPUCompositeAlphaMode_Auto;
  }

  wgpuSurfaceCapabilitiesFreeMembers(caps);

  wgpuAdapterRelease(adapter);

  std::cout << "[Husk GPU] WebGPU Pipeline successfully initialized."
            << std::endl;
}

void WgpuRenderer::configure_surface(int width, int height) {
  if (width == 0 || height == 0)
    return;

  WGPUSurfaceConfiguration config = {};
  config.nextInChain = nullptr;
  config.device = m_device;
  config.format = m_swapchain_format;
  config.usage = WGPUTextureUsage_RenderAttachment;
  config.width = width;
  config.height = height;
  config.presentMode = WGPUPresentMode_Fifo;
  config.alphaMode = m_alpha_mode;

  wgpuSurfaceConfigure(m_surface, &config);
}

void WgpuRenderer::resize(int width, int height) {
  configure_surface(width, height);
}

void WgpuRenderer::draw_frame() {
  if (!m_surface)
    return;

  WGPUSurfaceTexture surfaceTexture;
  wgpuSurfaceGetCurrentTexture(m_surface, &surfaceTexture);

  if (!surfaceTexture.texture) {
    return;
  }

  WGPUTextureViewDescriptor viewDesc = {};
  viewDesc.format = wgpuTextureGetFormat(surfaceTexture.texture);
  viewDesc.dimension = WGPUTextureViewDimension_2D;
  viewDesc.baseMipLevel = 0;
  viewDesc.mipLevelCount = 1;
  viewDesc.baseArrayLayer = 0;
  viewDesc.arrayLayerCount = 1;
  viewDesc.aspect = WGPUTextureAspect_All;
  WGPUTextureView targetView =
      wgpuTextureCreateView(surfaceTexture.texture, &viewDesc);

  WGPUCommandEncoderDescriptor encoderDesc = {};
  WGPUCommandEncoder encoder =
      wgpuDeviceCreateCommandEncoder(m_device, &encoderDesc);

  WGPURenderPassColorAttachment colorAttachment = {};
  colorAttachment.view = targetView;
  colorAttachment.loadOp = WGPULoadOp_Clear;
  colorAttachment.storeOp = WGPUStoreOp_Store;
  colorAttachment.clearValue = WGPUColor{0.141f, 0.141f, 0.224f, 1.0f};

#ifndef WGPU_DEPTH_SLICE_UNDEFINED
#define WGPU_DEPTH_SLICE_UNDEFINED ~0U
#endif
  colorAttachment.depthSlice = WGPU_DEPTH_SLICE_UNDEFINED;

  WGPURenderPassDescriptor renderPassDesc = {};
  renderPassDesc.colorAttachmentCount = 1;
  renderPassDesc.colorAttachments = &colorAttachment;

  WGPURenderPassEncoder renderPass =
      wgpuCommandEncoderBeginRenderPass(encoder, &renderPassDesc);
  wgpuRenderPassEncoderEnd(renderPass);
  wgpuRenderPassEncoderRelease(renderPass);

  WGPUCommandBufferDescriptor cmdBufDesc = {};
  WGPUCommandBuffer command = wgpuCommandEncoderFinish(encoder, &cmdBufDesc);
  wgpuCommandEncoderRelease(encoder);

  wgpuQueueSubmit(m_queue, 1, &command);
  wgpuCommandBufferRelease(command);

  wgpuSurfacePresent(m_surface);
  wgpuTextureViewRelease(targetView);
}

} // namespace husk::client::render
