use crate::{
    ApplicationExtension, ColorTable, CommentExtension, Context, DataSubBlock, DataSubBlocks,
    ExtensionBlock, GifData, GraphicControlExtension, GraphicRenderingBlock, ImageDescriptor,
    LogicalScreenDescriptor, PlainTextExtension, TableBasedImageData, Version, TRAILER,
};
use anyhow::{anyhow, bail};
use log::{debug, error, info};

pub fn decode(bytes: &[u8], discard_comments: bool) -> anyhow::Result<GifData> {
    let mut cx = Context::default();

    if bytes[cx.offset] != b'G' {
        bail!(
            "invalid signature at offset {}, expected b'G', received '{}'",
            cx.offset,
            bytes[cx.offset]
        );
    }
    cx.offset += 1;
    if bytes[cx.offset] != b'I' {
        bail!(
            "invalid signature at offset {}, expected b'I', received '{}'",
            cx.offset,
            bytes[cx.offset]
        );
    }
    cx.offset += 1;
    if bytes[cx.offset] != b'F' {
        bail!(
            "invalid signature at offset {}, expected b'F', received '{}'",
            cx.offset,
            bytes[cx.offset]
        );
    }
    cx.offset += 1;

    let version = Version::decode(&mut cx, bytes)?;
    info!("GIF version: {:?}", version);

    let logical_screen_descriptor = LogicalScreenDescriptor::decode(&mut cx, bytes);
    info!("Found logical screen descriptor");
    debug!(
        "[{:?}] Logical screen descriptor: {:?}",
        cx, logical_screen_descriptor
    );

    let mut application_extensions = Vec::new();
    let mut graphic_rendering_blocks = Vec::new();
    let mut comment_extensions = Vec::new();
    loop {
        match bytes[cx.offset] {
            ExtensionBlock::INTRODUCER => {
                cx.offset += 1;
                if let Some(extension_block) =
                    ExtensionBlock::decode(&mut cx, bytes, discard_comments)?
                {
                    let extension_block_s: &'static str = (&extension_block).into();
                    info!("Found {} extension block", extension_block_s);
                    debug!("[{:?}] Extension block: {:?}", cx, extension_block);

                    match extension_block {
                        ExtensionBlock::GraphicControl(ext) => {
                            cx.graphic_control_extension = Some(ext);
                        }
                        ExtensionBlock::Application(ext) => {
                            application_extensions.push(ext);
                        }
                        ExtensionBlock::PlainText(ext) => {
                            if logical_screen_descriptor.global_color_table.is_none() {
                                error!("Discarding plain text extension block because no global color table is available");
                                continue;
                            }
                            graphic_rendering_blocks.push(GraphicRenderingBlock::PlainText(ext));
                        }
                        ExtensionBlock::Comment(ext) => {
                            comment_extensions.push(ext);
                        }
                    }
                }
            }
            ImageDescriptor::SEPARATOR => {
                cx.offset += 1;
                let image_descriptor = ImageDescriptor::decode(&mut cx, bytes);
                info!("Found image descriptor");
                debug!("[{:?}] Image descriptor: {:?}", cx, image_descriptor);
                graphic_rendering_blocks.push(GraphicRenderingBlock::Image(image_descriptor));
            }
            TRAILER => {
                info!("End of GIF data stream");
                break;
            }
            b => bail!("unknown byte 0x{:x} at offset {}", b, cx.offset),
        }
    }

    Ok(GifData {
        version,
        logical_screen_descriptor,
        application_extensions,
        graphic_rendering_blocks,
        comment_extensions,
    })
}

impl Version {
    fn decode(cx: &mut Context, version: &[u8]) -> anyhow::Result<Self> {
        let s = match &version[cx.offset..cx.offset + 3] {
            b"87a" => Ok(Self::V87a),
            b"89a" => Ok(Self::V89a),
            v => Err(anyhow!(
                "invalid GIF version at offset {}. Expected either b\"v87a\" or b\"89a\", got '{:?}'",
                cx.offset,
                v
            )),
        };
        cx.offset += 3;
        s
    }
}

impl<'a> LogicalScreenDescriptor<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Self {
        let logical_screen_width = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let logical_screen_height = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let packed_fields = bytes[cx.offset];
        cx.offset += 1;
        let background_color_index = bytes[cx.offset];
        cx.offset += 1;
        let pixel_aspect_ratio = bytes[cx.offset];
        cx.offset += 1;
        let mut s = Self {
            logical_screen_width,
            logical_screen_height,
            packed_fields,
            background_color_index,
            pixel_aspect_ratio,
            global_color_table: None,
        };

        if s.global_color_table_flag() == 1 {
            let global_color_table_size = 3 * 2usize.pow(s.global_color_table_size() as u32 + 1u32);
            info!(
                "Global color table found: size -> {}, number of pixels -> {}",
                global_color_table_size,
                global_color_table_size / 3
            );
            s.global_color_table = Some(ColorTable {
                pixels: &bytes[cx.offset..cx.offset + global_color_table_size],
            });
            debug!("[{:?}] Global color table: {:?}", cx, s.global_color_table);
            cx.offset += global_color_table_size;
        };
        s
    }
}

impl<'a> ExtensionBlock<'a> {
    fn decode(
        cx: &mut Context,
        bytes: &'a [u8],
        discard_comments: bool,
    ) -> anyhow::Result<Option<Self>> {
        let label = bytes[cx.offset];
        cx.offset += 1;
        match label {
            GraphicControlExtension::LABEL => Ok(Some(Self::GraphicControl(
                GraphicControlExtension::decode(cx, bytes)?,
            ))),
            CommentExtension::LABEL => {
                if discard_comments {
                    Ok(None)
                } else {
                    Ok(Some(Self::Comment(CommentExtension::decode(cx, bytes))))
                }
            }
            PlainTextExtension::LABEL => Ok(Some(Self::PlainText(PlainTextExtension::decode(
                cx, bytes,
            )?))),
            ApplicationExtension::LABEL => Ok(Some(Self::Application(
                ApplicationExtension::decode(cx, bytes)?,
            ))),
            label => Err(anyhow!(
                "invalid extension block label '{}' at offset {}",
                label,
                cx.offset
            )),
        }
    }
}

impl<'a> DataSubBlock<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Option<Self> {
        let block_size = bytes[cx.offset];
        cx.offset += 1;
        if block_size == Self::BLOCK_TERMINATOR {
            return None;
        }
        let data = &bytes[cx.offset..cx.offset + block_size as usize];
        cx.offset += block_size as usize;
        Some(Self { block_size, data })
    }
}

impl<'a> DataSubBlocks<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Self {
        let mut blocks = Vec::new();
        loop {
            if let Some(block) = DataSubBlock::decode(cx, bytes) {
                blocks.push(block);
            } else {
                break;
            }
        }
        Self { blocks }
    }
}

impl<'a> ApplicationExtension<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> anyhow::Result<Self> {
        let block_size = bytes[cx.offset];
        if block_size != Self::BLOCK_SIZE {
            bail!(
                "invalid block size at offset {}: expected '{}', got '{}'",
                cx.offset,
                Self::BLOCK_SIZE,
                block_size
            );
        }
        cx.offset += 1;
        let identifier = &bytes[cx.offset..cx.offset + 8];
        cx.offset += 8;
        let authentication_code = &bytes[cx.offset..cx.offset + 3];
        cx.offset += 3;
        let data = DataSubBlocks::decode(cx, bytes);
        Ok(Self {
            identifier,
            authentication_code,
            data,
        })
    }
}

impl<'a> PlainTextExtension<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> anyhow::Result<Self> {
        let block_size = bytes[cx.offset];
        if block_size != Self::BLOCK_SIZE {
            bail!(
                "invalid block size at offset {}: expected '{}', got '{}'",
                cx.offset,
                Self::BLOCK_SIZE,
                block_size
            );
        }
        cx.offset += 1;
        let text_grid_left_position =
            ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let text_grid_top_position = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let text_grid_width = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let text_grid_height = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let character_cell_width = bytes[cx.offset];
        cx.offset += 1;
        let character_cell_height = bytes[cx.offset];
        cx.offset += 1;
        let text_foreground_color_index = bytes[cx.offset];
        cx.offset += 1;
        let text_background_color_index = bytes[cx.offset];
        cx.offset += 1;
        let data = DataSubBlocks::decode(cx, bytes);
        Ok(Self {
            text_grid_left_position,
            text_grid_top_position,
            text_grid_width,
            text_grid_height,
            character_cell_width,
            character_cell_height,
            text_foreground_color_index,
            text_background_color_index,
            data,
            graphic_control_extension: cx.graphic_control_extension.take(),
        })
    }
}

impl GraphicControlExtension {
    fn decode(cx: &mut Context, bytes: &[u8]) -> anyhow::Result<Self> {
        let block_size = bytes[cx.offset];
        if block_size != Self::BLOCK_SIZE {
            bail!(
                "invalid block size at offset {}: expected '{}', got '{}'",
                cx.offset,
                Self::BLOCK_SIZE,
                block_size
            );
        }
        cx.offset += 1;
        let packed_fields = bytes[cx.offset];
        cx.offset += 1;
        let delay_time = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let transparent_color_index = bytes[cx.offset];
        cx.offset += 1;
        if bytes[cx.offset] != DataSubBlock::BLOCK_TERMINATOR {
            bail!(
                "invalid block terminator at offset {}: expected '{}', got '{}'",
                cx.offset,
                DataSubBlock::BLOCK_TERMINATOR,
                bytes[cx.offset]
            );
        }
        cx.offset += 1;
        Ok(Self {
            packed_fields,
            delay_time,
            transparent_color_index,
        })
    }
}

impl<'a> CommentExtension<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Self {
        let data = DataSubBlocks::decode(cx, bytes);
        Self { data }
    }
}

impl<'a> ImageDescriptor<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Self {
        let image_left_position = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let image_top_position = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let image_width = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let image_height = ((bytes[cx.offset] as u16) << 8) | bytes[cx.offset + 1] as u16;
        cx.offset += 2;
        let packed_fields = bytes[cx.offset];
        cx.offset += 1;

        let local_color_table_flag = packed_fields >> 7;
        let local_color_table = if local_color_table_flag == 1 {
            let local_color_table_size = (packed_fields << 5) >> 5;
            let local_color_table_size = 3 * 2usize.pow(local_color_table_size as u32 + 1u32);
            info!(
                "Local color table found: size -> {}, number of pixels -> {}",
                local_color_table_size,
                local_color_table_size / 3
            );
            let local_color_table = Some(ColorTable {
                pixels: &bytes[cx.offset..cx.offset + local_color_table_size],
            });
            debug!("[{:?}] Local color table: {:?}", cx, local_color_table);
            cx.offset += local_color_table_size;
            local_color_table
        } else {
            None
        };

        let image_data = TableBasedImageData::decode(cx, bytes);

        Self {
            image_left_position,
            image_top_position,
            image_width,
            image_height,
            packed_fields,
            local_color_table,
            image_data,
            graphic_control_extension: cx.graphic_control_extension.take(),
        }
    }
}

impl<'a> TableBasedImageData<'a> {
    fn decode(cx: &mut Context, bytes: &'a [u8]) -> Self {
        let lzw_minimum_code_size = bytes[cx.offset];
        cx.offset += 1;
        let image_data = DataSubBlocks::decode(cx, bytes);
        Self {
            lzw_minimum_code_size,
            image_data,
        }
    }
}
