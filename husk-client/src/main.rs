use futures::{SinkExt, StreamExt};
use husk_core::ipc::{ClientCodec, ClientMessage, DaemonMessage, get_socket_path};
use std::process::Command;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

// ==========================================
// Custom Events for Event Loop
// ==========================================

#[derive(Debug)]
enum AppEvent {
    IpcReceived(DaemonMessage),
    IpcError(String),
}

// ==========================================
// Main Function
// ==========================================

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Husk Client starting...");

    // Setup event loop with custom event support
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");
    let proxy = event_loop.create_proxy();

    // Setup channel for communication from UI to IPC thread
    let (ui_tx, ipc_rx) = mpsc::channel::<ClientMessage>();

    // Spawn background Tokio thread for IPC broker
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        rt.block_on(async {
            if let Err(e) = run_ipc_broker(proxy.clone(), ipc_rx).await {
                let _ = proxy.send_event(AppEvent::IpcError(e.to_string()));
            }
        });
    });

    // Share window ownership using Arc to satisfy both wgpu surface and winit closure requirements
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Husk Terminal")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
            .build(&event_loop)
            .expect("Failed to create window"),
    );

    // Initialize wgpu render state
    let mut render_state = pollster::block_on(RenderState::new(window.clone()));
    let mut has_received_dummy_frame = false;

    // Run the winit event loop
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::UserEvent(app_event) => match app_event {
                    AppEvent::IpcReceived(DaemonMessage::HandshakeAck) => {
                        tracing::info!("UI Thread: Handshake Acknowledged");
                    }
                    AppEvent::IpcReceived(DaemonMessage::DummyFrame { epoch, pixels }) => {
                        tracing::info!(
                            "UI Thread: Received DummyFrame (Epoch: {}, Pixels: {} bytes)",
                            epoch,
                            pixels.len()
                        );
                        // Trigger redraw to update background color
                        has_received_dummy_frame = true;
                        window.request_redraw();
                    }
                    AppEvent::IpcError(err) => {
                        tracing::error!("IPC Thread crashed: {}", err);
                        elwt.exit();
                    }
                    _ => {}
                },
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(physical_size) => {
                            render_state.resize(physical_size);
                            // Send resize dimensions to daemon
                            let _ = ui_tx.send(ClientMessage::Resize {
                                cols: 80,
                                rows: 24,
                                px_width: physical_size.width,
                                px_height: physical_size.height,
                                epoch: 1,
                            });
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    logical_key,
                                    ..
                                },
                            ..
                        } => {
                            // Exit application on Escape key
                            if logical_key == Key::Named(NamedKey::Escape) {
                                let _ = ui_tx.send(ClientMessage::Detach);
                                elwt.exit();
                            } else {
                                // Send dummy input to daemon on any other key press
                                tracing::debug!("Key pressed. Sending dummy Input to Daemon.");
                                let _ = ui_tx.send(ClientMessage::Input(vec![0x0D]));
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            render_state.render(has_received_dummy_frame);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .expect("Event loop crashed");
}

// ==========================================
// IPC Broker & Autospawn
// ==========================================

async fn run_ipc_broker(
    proxy: EventLoopProxy<AppEvent>,
    ipc_rx: mpsc::Receiver<ClientMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    let mut retries = 0;

    // Attempt connection with automatic daemon spawning on failure
    let stream = loop {
        match UnixStream::connect(&socket_path).await {
            Ok(s) => break s,
            Err(_) => {
                if retries == 0 {
                    tracing::warn!("Daemon not found. Attempting Autospawn...");
                    autospawn_daemon();
                } else if retries > 10 {
                    return Err("Failed to connect to Daemon after multiple attempts".into());
                }
                retries += 1;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    };

    tracing::info!("Successfully connected to Daemon UDS.");
    let mut framed = Framed::new(stream, ClientCodec::new());

    // Perform initial handshake
    framed
        .send(ClientMessage::Handshake { session_id: 1024 })
        .await?;

    let (mut sink, mut stream) = framed.split();

    // Spawn future to read from UDS and forward to UI event loop
    let proxy_clone = proxy.clone();
    let read_future = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if proxy_clone.send_event(AppEvent::IpcReceived(msg)).is_err() {
                break;
            }
        }
    });

    // Spawn future to read from UI channel and forward to UDS
    let write_future = tokio::spawn(async move {
        loop {
            if let Ok(msg) = ipc_rx.try_recv() {
                if sink.send(msg).await.is_err() {
                    break;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    });

    tokio::select! {
        _ = read_future => tracing::warn!("IPC Read future terminated"),
        _ = write_future => tracing::warn!("IPC Write future terminated"),
    }

    Ok(())
}

fn autospawn_daemon() {
    // Determine the path to the daemon executable
    let mut exe_path = std::env::current_exe().expect("Failed to get current exe path");
    exe_path.set_file_name("husk-daemon");

    let mut cmd = if exe_path.exists() {
        Command::new(exe_path)
    } else {
        // Fallback to cargo run for development environment
        let mut c = Command::new("cargo");
        c.args(["run", "--bin", "husk-daemon"]);
        c
    };

    match cmd.spawn() {
        Ok(_) => tracing::info!("Daemon process launched successfully in background."),
        Err(e) => tracing::error!("Failed to spawn daemon: {}", e),
    }
}

// ==========================================
// WGPU Renderer State
// ==========================================

struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl RenderState {
    async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        // Initialize wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find wgpu adapter");

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .expect("Failed to create wgpu device");

        // Configure surface with optimal format
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // Reconfigure surface on resize
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self, has_received_dummy: bool) {
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => {
                self.resize(self.size);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("Out of memory when rendering"),
            Err(e) => {
                tracing::warn!("Surface error: {:?}", e);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Determine background color based on dummy frame reception
        let clear_color = if has_received_dummy {
            wgpu::Color {
                r: 0.1,
                g: 0.8,
                b: 0.2,
                a: 1.0,
            }
        } else {
            wgpu::Color {
                r: 0.05,
                g: 0.05,
                b: 0.05,
                a: 1.0,
            }
        };

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
