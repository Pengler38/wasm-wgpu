#[allow(dead_code)]
pub fn print(string: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        wgpu::web_sys::console::log_1(&string.into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("{}", string);
    }
}

pub fn instance_descriptor() -> wgpu::InstanceDescriptor {
    #[cfg(target_arch = "wasm32")]
    {
        wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            //backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        wgpu::InstanceDescriptor::default()
    }
}

pub fn device_descriptor<'a>() -> wgpu::DeviceDescriptor<'a> {
    #[cfg(target_arch = "wasm32")]
    {
        wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            ..Default::default()
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        wgpu::DeviceDescriptor {
            ..Default::default()
        }
    }
}



//#[cfg(target_arch = "wasm32")]
//use winit::platform::web::WindowAttributesExtWebSys;
//#[cfg(target_arch = "wasm32")]
//use web_sys::wasm_bindgen::JsCast;
pub fn window_attributes() -> winit::window::WindowAttributes {
    #[cfg(target_arch = "wasm32")]
    {
        //Get Canvas, add to window attributes
        use winit::platform::web::WindowAttributesExtWebSys;
        use wgpu::web_sys::wasm_bindgen::JsCast;
        let canvas = wgpu::web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("canvas"))
            .map(|e| e.dyn_into::<wgpu::web_sys::HtmlCanvasElement>().unwrap())
            .unwrap(); //Final unwrap to make sure the canvas exists!!

        winit::window::WindowAttributes::default().with_canvas(Some(canvas))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let size = winit::dpi::PhysicalSize {
            width: 1280,
            height: 320,
        };
        winit::window::WindowAttributes::default().with_title("Test").with_inner_size(size)
    }
}
