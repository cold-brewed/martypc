
## [0.2.0](https://github.com/dbalsom/martypc/releases/tag/0.2.0) (2024-XX-XX)

### New Features

* #### New Display Manager Framework
  * New graphics backend abstraction layer will eventually allow frontends built for other graphics backends such as SDL
  * New scaling modes: Choose from Fixed, Integer, Fit and Stretch scaling options
  * Display Apertures: Choose how much of the emulated display field you wish to see:
    * No overscan
    * 'Monitor-accurate' overscan
    * Full overscan
    * Full display field including hblank and vblank periods (for debugging)
  * Hardware aspect correction: Aspect correction is now performed in the shader for reduced CPU load
  * Multi-window support: Define additional output windows in the configuration file
    * Multiple windows can display the same video card, with different display options
    * Windows can be set to a fixed size or scale factor, or made completely resizable
    * Windows can be pinned 'always on top'
    * Windows can be toggled full-screen via CONTROL-ALT-ENTER hotkey 
  * Shader support: A basic CRT shader is built into the new display scaler
    * Internal shader features:
      * Barrel distortion
      * Corner radius (rounded corners)
      * Monochrome phosphor simulation
      * Scanline emulation synchronized to emulated video resolution  
    * Presets for the internal scaler can be defined in configuration and applied to different windows

* #### New ROM Definition Framework
  * ROM set definitions are no longer hardcoded. They will be read from ROM set definition TOML files in the ROM directory.
  * Add your own custom ROM sets or contribute missing ROM definitions to MartyPC!

* #### New Machine Configuration Framework
  * Define multiple Machine hardware configurations, name them, select them in the main config or via command line argument.
  * Configure the amount of conventional memory, the number and type of floppy drives, video cards, serial ports and more!

* #### New Resource Manager Framework
  * Paths are to 'resource ids' are now fully configurable, and facilities to build file trees are provided. 
    MartyPC can create needed directories on startup. The Floppy and VHD browsers have been rewritten to take
advantage of this new system, and so now you can organize your media directories into subdirectories for convenience.
  
* #### EGA Video Card
  * EGA is back! A character-clocked EGA implementation is here, although it may still be a bit rough around the edges. 
    EGA will continue to be polished in upcoming releases.
  * Features:
    * Functional emulation of each of the 5 LSI chips on the EGA
    * Per-scanline Pel panning
    * Line compare register - See the status bar in Catacombs 3d!
    * CGA compatibility mode - Play Alleycat!
    * Software fonts - change your DOS font, or see a graphical mouse cursor in text mode (Norton Utilities 6.0)
  * Known issues:
    * Some paging issues (flickering sprites in Dangerous Dave)
    * Some scrolling issues (jerky motion in Commander Keen 4)
    * Some more obscure registers not properly emulated / investigated (SOM, etc)
    * Aperture defintions / adjustments not final
    * Implementation may be slow / unoptimized in parts
    
* #### MDA Video Card
  * Not quite as a flashy as EGA, but the MDA card type is now also supported, and moreover, you can install an MDA 
    alongside a CGA or EGA card for a dual head display.
  * 9th column rendering and underline attributes supported
  * Includes the framework for a LPT port, which will now be detected
  * Known issues:
    * Needs optimization - due to the 9-dot character clock making 64-bit aligned writes impossible, MDA is currently slower to emulate than EGA.
* #### New Keyboard System
  * MartyPC now performs low-level emulation of a Model F keyboard instead of directly translating OS input events to the core
    * Model M emulation to come
  * Guest typematic rate is fully configurable
  * International keyboard layouts are now supported via translation files.
    * Translation files support all keycode names defined by w3c: [https://w3c.github.io/uievents-code/#code-value-tables](https://w3c.github.io/uievents-code/#code-value-tables)
    * Translation files can define direct scancode mappings or full macros

### Debugger/GUI Improvements
* Reorganized Debug menu
* Improved appearance of CPU State display
* Instruction Trace: In Csv trace mode, instruction trace now has a scrolling table with optional columns
* Memory Viewer: Fixed scrolling issues, disassembly popup now uses fixed-width font
* IVT Viewer: Fixed scrolling, IVT entries now animate on change, added annotations
* Instruction History - fields now align with Disassembly View, and cycle counts have been moved to the right
* Memory Viewer will now show values for memory mapped regions
* Improved VHD creator - should no longer be confusing to use
* Text Mode Viewer - View ASCII contents of video memory, which you can select and copy to clipboard
* New themes courtesy of [egui-themer crate](https://github.com/grantshandy/egui-themer)
* New notification system courtesy of [egui-notify crate](https://github.com/ItsEthra/egui-notify). 
  * Implemented success/error notifications for disk and file operations, screenshots, etc.

### Frontend Bug Fixes / Improvements
* Floppy and HDD browsers now support subdirectories
* Write protection can be toggled for floppy drives
* Sound initialization is now optional
* Added 8088 JSON CPU test generator and validator 
  * Used to create the first [comprehensive test suite for the Intel 8088](https://github.com/TomHarte/ProcessorTests/tree/main/8088)
* Added debug_keyboard config flag - this will print keyboard event info to the console for support

### Core Bug Fixes / Improvements

* CPU: Refactor PC from u32 to u16 to address segment wrapping issues, implement ip() in terms of PC
* CPU: Instruction counts properly increment even when instruction history is off
* CPU: Fixed device ticks after interrupts
* CPU: Improved Halt/Resume logic and cycle timings
* CPU: New sigrok cycle log format for viewing cycle logs in sigrok PulseView logic analyzer
* CPU: Updated disassembler to normalize output against iced-x86. Now resolves negative immediates and displacements.
* CPU: Fixed typo for 'bp+di+DISP' in both disassemblers (Thanks Tom Harte)
* CPU: Brand new, simplified BIU state logic
* CPU: Fixed & Improved DMA refresh scheduling. (Fixes 8088MPH CPU test)
* CPU: Fixed issue where Call Stack could grow uncontrollably with recursive code or interrupts
* CPU: Fixed CS:IP reporting in Instruction trace mode logs
* CPU: Fixed memory leak in Instruction trace mode (thanks Folkert)
* CPU: Fixed CPU cycle timings for LES and LDS instructions
* CPU: Fixed CPU issue where incorrect microcode jump was listed for fixed word displacements
* CPU: Fixed CPU issue where a prefetch abort would not properly override a prefetch delay
* PIC: Ignore IMR during INTA pulse
* PIC: Handle multiple IRR bits unmaksed at the same time
* PIC: Honor IRQ offset specified in IWC2 to PIC (Thanks Folkert)
* CGA: Preliminary CGA snow emulation. Not yet 100% accurate
* CGA: Properly disable cursor if cursor start > maximum scanline
* CGA: Reverted color palette entry for black from dark gray to true black
* CGA: Fully reset the CGA device on reboot. May(?) fix issue with black screens in 8088MPH. (Thanks hirudov)
* CGA: Don't recalculate composite parameters if mode change was enable bit only
* Xebec HDC: Proceed from Reset state to WaitngForCommand after delay
* Xebec HDC: Implemented missing Read Sector Buffer command (Fixes panic in IBM diagnostics)

### Major dependency updates:
  * wgpu to 0.18 (latest)
  * egui to 0.24.2 (latest, forked)
  * pixels to 0.13 (latest, forked)
  * winit to 0.29.4 (latest)

## [0.1.3](https://github.com/dbalsom/martypc/releases/tag/0.1.3) (2023-07-06)

* Disabled window doubling if window would not fit on screen due to DPI scaling.
* Decreased minimum window size
* Disabled warpspeed config flag
* Renamed 'autostart' config flag to 'auto_poweron' and fixed poweron logic.
* Mapped Right Alt, Control and Shift to emulated Left Alt, Control and Shift.
* Added UI warning when MartyPC is compiled in debug mode (unfortunately the default behavior of cargo build)
* Replaced CGA composite simulation code with reenigne's color multiplexer algorithm, for more accurate colors and a 3x speed improvement.
* Added contrast, phase and CGA type controls to composite adjustment window.
* Update Pixels to 0.13
* Update egui and egui-wgpu 0.22
* Update winit to 0.29* 
    * Winit 0.29 fixes reported keyboard issues with non-US keyboard layouts unable to type certain keys.
    * Winit 0.29 fixes excessively high updates per second on some Linux builds
    * Enabled Wayland support
* Enable basic clipboard integration in debugger for copy/paste of breakpoints. This feature will be expanded.
* Fork egui-winit 0.22 to manually update winit dependency to 0.29.

## [0.1.2](https://github.com/dbalsom/martypc/releases/tag/0.1.2) (2023-06-29)

* Relicensed MartyPC under the MIT license.
* Redesigned CGA card with 'dynamic clocking' support. Card will now switch between clocking by cycle or character as appropriate.
* Improved hsync logic, screens in all graphics modes are now horizontally centered properly.
* Added 1.44MB floppy image definition. Somehow, these are readable(!?) (thanks xcloudplatform for discovering this)
* Fixed CGA palette handling bug. Fixes California Games CGAMORE mode. (thanks VileR)
* Added short tick delay between writing PIC IMR and raising any unmasked IRR bit to INTR. Fixes halts on warm boot.
* Improved performance when CPU is halted.
* Added menu options to save changes to loaded floppy image(s).
* Fixed CPU cycle tracelogging
* Added port mirrors for CGA (thanks th3bar0n)
* Fixed address wrapping for graphics modes (thanks th3bar0n)
* Fixed handling of mode enable flag in text mode (thanks VileR)
* Implemented better composite adjustment defaults (Matches colors in 8088mph better)
* Switched from cgmath to glam vector library. Approx 30% speedup in CGA composite simulation.
* Utilized bytemuck crate to write 32 bits at a time for CGA index->RGBA conversion, about 3x performance improvement
* Reorganized project structure. Refactored emulator core to Rust library and frontend components.
* Added Criterion for benchmarking components.
* Update Pixels library to 0.12.1
* Use fast_image_resize crate for SIMD acceleration. Aspect correction is now approximately 5X faster with equivalent quality.
* Fixed inaccuracy in keyboard shift register handling 
* Fixed bug in PIT latch logic (thanks 640KB)
* Fixed bug in PIC IRR logic (thanks 640KB)
* Fixed bug in PPI handling of keyboard enable line (Fixes halt on boot on 5160)
* Added CTRL-ALT-DEL menu option
* Known issues:
    * Turbo mode may cause the IBM BIOS to halt during POST during PIT checkout.
    * Formatting floppies is limited to 360K due to fixed drive type. 
    * Regression: PIT latch logic change has now made 8088MPH report a 1% CPU variation. I believe this is more a timer issue than a CPU issue.

## [0.1.1](https://github.com/dbalsom/martypc/releases/tag/0.1.1) (2023-05-31)

* Compiled for CGA only
* Fixed CGA cursor handling
* Rescan media folders when opening Media menu
* Added barebones documentation
* Added icon resource for Windows build
* Added ROM override feature
* Added HDD drive1 functionality
* Known issues
    * Floppy images are read-only.

## [0.1.0](https://github.com/dbalsom/martypc/releases/tag/0.1.0) (2023-05-29)

* Limited testing preview
