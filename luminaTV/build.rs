use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=cxx/bridge.cpp");
    println!("cargo:rerun-if-changed=cxx/bridge.h");

    // NDI SDK linking configuration
    let sdk_dir = if let Ok(dir) = env::var("NDI_SDK_DIR") {
        PathBuf::from(dir)
    } else {
        // Try common default locations for macOS
        #[cfg(target_os = "macos")]
        {
            let possible_paths = vec![
                "/Library/NDI SDK for macOS",
                "/Library/NDI SDK for Apple",
                "/Library/NDI 6 SDK",
                "/Library/NDI SDK",
                "/Library/NewTek/NDI SDK",
                "/Library/Application Support/NDI SDK for Apple",
                "/Applications/NDI SDK for Apple",
                "/Applications/NDI 6 SDK",
            ];

            possible_paths
                .into_iter()
                .find(|p| Path::new(p).exists())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    panic!("Unsupported OS or NDI_SDK_DIR not set. Please set NDI_SDK_DIR environment variable or install NDI SDK to a common location.");
                })
        }
        #[cfg(not(target_os = "macos"))]
        {
            panic!("Unsupported OS. Please set NDI_SDK_DIR environment variable.");
        }
    };

    let lib_dir = sdk_dir.join("lib/macOS");
    let include_dir = sdk_dir.join("include");

    // Add the library directory to the search path
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Link the appropriate NDI library
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=dylib=ndi_advanced");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        // Add Swift runtime rpath for ScreenCaptureKit
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx");
    }

    // Generate the bindings using bindgen.
    let bindings = bindgen::Builder::default()
        .header(include_dir.join("Processing.NDI.Lib.h").to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new())) // Corrected usage
        .generate()
        .expect("Unable to generate NDI bindings");

    // Write the bindings to the $OUT_DIR/ndi_bindings.rs file.
    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR environment variable not set"));
    bindings
        .write_to_file(out_path.join("ndi_bindings.rs"))
        .expect("Couldn't write NDI bindings!");

    // Compile Slint UI
    slint_build::compile("ui/app.slint").unwrap();

    // CXX bridge build
    cxx_build::bridge("src/main.rs")
        .file("cxx/bridge.cpp")
        .flag_if_supported("-std=c++17")
        .compile("luminatv-cxx");

    // DeckLink SDK configuration
    // Determine SDK path: environment variable > common fallback > project relative path
    let decklink_sdk_dir = if let Ok(dir) = env::var("DECKLINK_SDK_DIR") {
        PathBuf::from(dir)
    } else {
         // Project relative path (libs/decklink-sdk/Mac)
         // Current dir is package root (Monitor3G/luminaTV) -> ../libs/decklink-sdk/Mac
         let relative_sdk = PathBuf::from("../libs/decklink-sdk/Mac");
         if relative_sdk.exists() {
             relative_sdk.canonicalize().expect("Failed to canonicalize SDK path")
         } else {
             // Fallback to commonly used location or error out nicely if not found
             let common_path = PathBuf::from("/Library/Application Support/Blackmagic Design/Blackmagic DeckLink SDK/Mac");
             if common_path.exists() {
                 common_path
             } else {
                 // Try user-specific location found earlier
                 let user_path = PathBuf::from("/Users/bongpark/Downloads/Blackmagic DeckLink SDK 15.3/Mac");
                 if user_path.exists() {
                     user_path
                 } else {
                     panic!("DeckLink SDK not found. Please set DECKLINK_SDK_DIR or place SDK in ../libs/decklink-sdk/Mac");
                 }
             }
         }
    };
    
    let decklink_include = decklink_sdk_dir.join("include");
    println!("cargo:rerun-if-changed={}", decklink_include.display());
    
    // Get macOS SDK path
    let sdk_path = std::process::Command::new("xcrun")
        .args(&["--show-sdk-path"])
        .output()
        .expect("Failed to get macOS SDK path")
        .stdout;
    let sdk_path = String::from_utf8(sdk_path).unwrap().trim().to_string();

    // Compile DeckLinkAPIDispatch.cpp
    cc::Build::new()
        .cpp(true)
        .file(decklink_include.join("DeckLinkAPIDispatch.cpp"))
        .include(&decklink_include)
        .flag("-fobjc-arc") 
        .flag(&format!("-isysroot{}", sdk_path))
        .compile("DeckLinkAPI");

    // Compile camera_delegate.m
    cc::Build::new()
        .file("src/capture/camera_delegate.m")
        .flag("-fobjc-arc")
        .compile("camera_delegate");

    // Compile virtual_display.m (CGVirtualDisplay bridge)
    cc::Build::new()
        .file("src/capture/virtual_display.m")
        .flag("-fobjc-arc")
        .compile("virtual_display");
        
    // Bindgen for DeckLink
    let output_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Wrapper header for bindgen
    let wrapper_path = output_path.join("decklink_wrapper.h");
    std::fs::write(&wrapper_path, format!("#include \"{}/DeckLinkAPI.h\"", decklink_include.display())).unwrap();

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg("-x")
        .clang_arg("objective-c++")
        .clang_arg("-std=c++14")
        .clang_arg(format!("-I{}", decklink_include.display()))
        .clang_arg(format!("-isysroot{}", sdk_path))
        .clang_arg(format!("-F{}/System/Library/Frameworks", sdk_path))
        .clang_arg("-fobjc-arc")
        .vtable_generation(true) 
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("IDeckLink.*")
        .allowlist_type("IEDeckLink.*")
        .allowlist_type("BMD.*")
        .allowlist_var("BMD.*") // Constants
        .generate()
        .expect("Unable to generate DeckLink bindings");

        
    bindings
        .write_to_file(output_path.join("decklink_bindings.rs"))
        .expect("Couldn't write DeckLink bindings!");
}

