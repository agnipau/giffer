pub mod decoder;
pub mod encoder;

use std::fmt;

#[derive(Debug, Default)]
pub(crate) struct Context {
    pub(crate) offset: usize,
    pub(crate) graphic_control_extension: Option<GraphicControlExtension>,
}

#[derive(Debug)]
pub struct GifData<'a> {
    pub version: Version,
    pub logical_screen_descriptor: LogicalScreenDescriptor<'a>,
    pub application_extensions: Vec<ApplicationExtension<'a>>,
    pub comment_extensions: Vec<CommentExtension<'a>>,
    pub graphic_rendering_blocks: Vec<GraphicRenderingBlock<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Version {
    V87a,
    V89a,
}

pub struct LogicalScreenDescriptor<'a> {
    pub logical_screen_width: u16,
    pub logical_screen_height: u16,
    packed_fields: u8,
    pub background_color_index: u8,
    pub pixel_aspect_ratio: u8,
    pub global_color_table: Option<ColorTable<'a>>,
}

impl<'a> LogicalScreenDescriptor<'a> {
    // 1 bit (MSB)
    pub fn global_color_table_flag(&self) -> u8 {
        self.packed_fields >> 7
    }

    // 3 bits
    pub fn color_resolution(&self) -> u8 {
        (self.packed_fields << 1) >> 5
    }

    // 1 bit
    pub fn sort_flag(&self) -> u8 {
        (self.packed_fields << 4) >> 7
    }

    // 3 bit (3 LSB)
    pub fn global_color_table_size(&self) -> u8 {
        (self.packed_fields << 5) >> 5
    }

    pub fn aspect_ratio(&self) -> f32 {
        (self.pixel_aspect_ratio as f32 + 15.0) / 64.0
    }
}

impl<'a> fmt::Debug for LogicalScreenDescriptor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogicalScreenDescriptor")
            .field("logical_screen_width", &self.logical_screen_width)
            .field("logical_screen_height", &self.logical_screen_width)
            .field("global_color_table_flag", &self.global_color_table_flag())
            .field("color_resolution", &self.color_resolution())
            .field("sort_flag", &self.sort_flag())
            .field("global_color_table_size", &self.global_color_table_size())
            .field("aspect_ratio", &self.aspect_ratio())
            .field("background_color_index", &self.background_color_index)
            .field("pixel_aspect_ratio", &self.pixel_aspect_ratio)
            .field("global_color_table", &self.global_color_table)
            .finish()
    }
}

#[derive(Debug)]
pub struct ColorTable<'a> {
    pixels: &'a [u8],
}

impl<'a> ColorTable<'a> {
    pub fn get_pixel(&self, idx: usize) -> &[u8] {
        &self.pixels[idx * 3..idx * 3 + 3]
    }
}

#[derive(Debug)]
pub(crate) enum ExtensionBlock<'a> {
    GraphicControl(GraphicControlExtension),
    Comment(CommentExtension<'a>),
    PlainText(PlainTextExtension<'a>),
    Application(ApplicationExtension<'a>),
}

impl<'a> ExtensionBlock<'a> {
    pub(crate) const INTRODUCER: u8 = 0x21;
}

impl<'a> From<&ExtensionBlock<'a>> for &'static str {
    fn from(x: &ExtensionBlock) -> &'static str {
        match x {
            ExtensionBlock::GraphicControl(_) => "graphic control",
            ExtensionBlock::Comment(_) => "comment",
            ExtensionBlock::PlainText(_) => "plain text",
            ExtensionBlock::Application(_) => "application",
        }
    }
}

#[derive(Debug)]
pub enum GraphicRenderingBlock<'a> {
    PlainText(PlainTextExtension<'a>),
    Image(ImageDescriptor<'a>),
}

#[derive(Debug)]
pub struct DataSubBlock<'a> {
    pub block_size: u8,
    pub data: &'a [u8],
}

impl<'a> DataSubBlock<'a> {
    pub(crate) const BLOCK_TERMINATOR: u8 = 0x00;
}

#[derive(Debug)]
pub struct DataSubBlocks<'a> {
    pub blocks: Vec<DataSubBlock<'a>>,
}

#[derive(Debug)]
pub struct ApplicationExtension<'a> {
    pub identifier: &'a [u8],
    pub authentication_code: &'a [u8],
    pub data: DataSubBlocks<'a>,
}

impl<'a> ApplicationExtension<'a> {
    pub(crate) const LABEL: u8 = 0xff;
    pub(crate) const BLOCK_SIZE: u8 = 11;
}

#[derive(Debug)]
pub struct PlainTextExtension<'a> {
    pub text_grid_left_position: u16,
    pub text_grid_top_position: u16,
    pub text_grid_width: u16,
    pub text_grid_height: u16,
    pub character_cell_width: u8,
    pub character_cell_height: u8,
    pub text_foreground_color_index: u8,
    pub text_background_color_index: u8,
    pub data: DataSubBlocks<'a>,
    pub graphic_control_extension: Option<GraphicControlExtension>,
}

impl<'a> PlainTextExtension<'a> {
    pub(crate) const LABEL: u8 = 0x01;
    pub(crate) const BLOCK_SIZE: u8 = 12;
}

pub struct GraphicControlExtension {
    packed_fields: u8,
    pub delay_time: u16,
    pub transparent_color_index: u8,
}

impl GraphicControlExtension {
    pub(crate) const LABEL: u8 = 0xf9;
    pub(crate) const BLOCK_SIZE: u8 = 4;
}

impl GraphicControlExtension {
    // 3 bits
    pub fn disposal_method(&self) -> u8 {
        (self.packed_fields << 3) >> 5
    }

    // 1 bit
    pub fn user_input_flag(&self) -> u8 {
        (self.packed_fields << 6) >> 7
    }

    // 1 bit (LSB)
    pub fn transparent_color_flag(&self) -> u8 {
        (self.packed_fields << 7) >> 7
    }
}

impl fmt::Debug for GraphicControlExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GraphicControlExtension")
            .field("disposal_method", &self.disposal_method())
            .field("user_input_flag", &self.user_input_flag())
            .field("transparent_color_flag", &self.transparent_color_flag())
            .field("delay_time", &self.delay_time)
            .field("transparent_color_index", &self.transparent_color_index)
            .finish()
    }
}

#[derive(Debug)]
pub struct CommentExtension<'a> {
    pub data: DataSubBlocks<'a>,
}

impl<'a> CommentExtension<'a> {
    pub(crate) const LABEL: u8 = 0xfe;
}

pub struct ImageDescriptor<'a> {
    pub image_left_position: u16,
    pub image_top_position: u16,
    pub image_width: u16,
    pub image_height: u16,
    packed_fields: u8,
    pub local_color_table: Option<ColorTable<'a>>,
    pub image_data: TableBasedImageData<'a>,
    pub graphic_control_extension: Option<GraphicControlExtension>,
}

impl<'a> ImageDescriptor<'a> {
    const SEPARATOR: u8 = 0x2c;

    pub fn local_color_table_flag(&self) -> u8 {
        self.packed_fields >> 7
    }

    pub fn interlace_flag(&self) -> u8 {
        (self.packed_fields << 1) >> 7
    }

    pub fn sort_flag(&self) -> u8 {
        (self.packed_fields << 2) >> 7
    }

    pub fn local_color_table_size(&self) -> u8 {
        (self.packed_fields << 5) >> 5
    }
}

impl<'a> fmt::Debug for ImageDescriptor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageDescriptor")
            .field("image_left_position", &self.image_left_position)
            .field("image_top_position", &self.image_top_position)
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .field("local_color_table_flag", &self.local_color_table_flag())
            .field("interlace_flag", &self.interlace_flag())
            .field("sort_flag", &self.sort_flag())
            .field("local_color_table_size", &self.local_color_table_size())
            .field("local_color_table", &self.local_color_table)
            .field("image_data", &self.image_data)
            .field("graphic_control_extension", &self.graphic_control_extension)
            .finish()
    }
}

#[derive(Debug)]
pub struct TableBasedImageData<'a> {
    pub lzw_minimum_code_size: u8,
    pub image_data: DataSubBlocks<'a>,
}

pub(crate) const SIGNATURE: &'static [u8] = b"GIF";

pub(crate) const TRAILER: u8 = 0x3b;
