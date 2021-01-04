use crate::{
    ApplicationExtension, CommentExtension, DataSubBlock, DataSubBlocks, ExtensionBlock, GifData,
    GraphicControlExtension, GraphicRenderingBlock, ImageDescriptor, LogicalScreenDescriptor,
    PlainTextExtension, TableBasedImageData, Version, TRAILER,
};
use log::{debug, info};

impl<'a> GifData<'a> {
    pub fn encode(&self, version: &Version, discard_comments: bool) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(b"GIF");
        info!("Wrote the GIF signature");
        data.extend_from_slice(version.encode());
        info!("Wrote the GIF version: {:?}", version);
        let logical_screen_descriptor = &self.logical_screen_descriptor.encode();
        data.extend_from_slice(logical_screen_descriptor);
        info!("Wrote the logical screen descriptor");
        debug!(
            "Logical screen descriptor: {:x?}",
            logical_screen_descriptor
        );

        if *version == Version::V87a {
            info!("Discarding {} application extensions because they're not supported in the target version '{:?}'", self.application_extensions.len(), version);
        } else {
            for ext in &self.application_extensions {
                let ext = &ext.encode();
                data.extend_from_slice(ext);
                info!("Wrote an application extension");
                debug!("Application extension: {:x?}", ext);
            }
        }
        if !discard_comments && *version != Version::V87a {
            for ext in &self.comment_extensions {
                let ext = &ext.encode();
                data.extend_from_slice(ext);
                info!("Wrote a comment extension");
                debug!("Comment extension: {:x?}", ext);
            }
        } else if *version == Version::V87a {
            info!("Discarding {} comment extensions because they're not supported in the target version '{:?}'", self.comment_extensions.len(), version);
        } else {
            info!(
                "Discarding {} comment extensions",
                self.comment_extensions.len()
            );
        }
        for block in &self.graphic_rendering_blocks {
            match block {
                GraphicRenderingBlock::Image(_) => {
                    let block_data = &block.encode(version);
                    data.extend_from_slice(block_data);
                    info!("Wrote an image descriptor");
                    debug!("Image descriptor: {:x?}", block_data);
                }
                GraphicRenderingBlock::PlainText(_) => {
                    if *version == Version::V87a {
                        info!("Discarding plain text extension because it's not supported in the target version '{:?}'", version);
                    } else {
                        let block_data = &block.encode(version);
                        data.extend_from_slice(block_data);
                        info!("Wrote a plain text extension");
                        debug!("Plain text extension: {:x?}", block_data);
                    }
                }
            }
        }
        data.push(TRAILER);
        info!("Wrote the GIF trailer");

        data
    }
}

impl Version {
    fn encode(&self) -> &'static [u8] {
        match self {
            Self::V87a => b"87a",
            Self::V89a => b"89a",
        }
    }
}

impl<'a> LogicalScreenDescriptor<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push((self.logical_screen_width >> 8) as u8);
        data.push(((self.logical_screen_width << 8) >> 8) as u8);
        data.push((self.logical_screen_height >> 8) as u8);
        data.push(((self.logical_screen_height << 8) >> 8) as u8);
        data.push(self.packed_fields);
        data.push(self.background_color_index);
        data.push(self.pixel_aspect_ratio);
        if let Some(global_color_table) = &self.global_color_table {
            data.extend_from_slice(global_color_table.pixels);
        }
        data
    }
}

impl<'a> GraphicRenderingBlock<'a> {
    fn encode(&self, version: &Version) -> Vec<u8> {
        match self {
            Self::PlainText(block) => block.encode(),
            Self::Image(block) => block.encode(version),
        }
    }
}

impl<'a> DataSubBlock<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.block_size);
        data.extend_from_slice(self.data);
        data
    }
}

impl<'a> DataSubBlocks<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for block in &self.blocks {
            data.extend_from_slice(&block.encode());
        }
        data.push(DataSubBlock::BLOCK_TERMINATOR);
        data
    }
}

impl<'a> ApplicationExtension<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(ExtensionBlock::INTRODUCER);
        data.push(Self::LABEL);
        data.push(Self::BLOCK_SIZE);
        data.extend_from_slice(self.identifier);
        data.extend_from_slice(self.authentication_code);
        data.extend_from_slice(&self.data.encode());
        data
    }
}

impl<'a> PlainTextExtension<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(ExtensionBlock::INTRODUCER);
        data.push(Self::LABEL);
        if let Some(graphic_control_extension) = &self.graphic_control_extension {
            data.extend_from_slice(&graphic_control_extension.encode());
        }
        data.push(Self::BLOCK_SIZE);
        data.push((self.text_grid_left_position >> 8) as u8);
        data.push(((self.text_grid_left_position << 8) >> 8) as u8);
        data.push((self.text_grid_top_position >> 8) as u8);
        data.push(((self.text_grid_top_position << 8) >> 8) as u8);
        data.push((self.text_grid_width >> 8) as u8);
        data.push(((self.text_grid_width << 8) >> 8) as u8);
        data.push((self.text_grid_height >> 8) as u8);
        data.push(((self.text_grid_height << 8) >> 8) as u8);
        data.push(self.character_cell_width);
        data.push(self.character_cell_height);
        data.push(self.text_foreground_color_index);
        data.push(self.text_background_color_index);
        data.extend_from_slice(&self.data.encode());
        data
    }
}

impl GraphicControlExtension {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(ExtensionBlock::INTRODUCER);
        data.push(Self::LABEL);
        data.push(Self::BLOCK_SIZE);
        data.push(self.packed_fields);
        data.push((self.delay_time >> 8) as u8);
        data.push(((self.delay_time << 8) >> 8) as u8);
        data.push(self.transparent_color_index);
        data.push(DataSubBlock::BLOCK_TERMINATOR);
        data
    }
}

impl<'a> CommentExtension<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(ExtensionBlock::INTRODUCER);
        data.push(Self::LABEL);
        data.extend_from_slice(&self.data.encode());
        data
    }
}

impl<'a> ImageDescriptor<'a> {
    fn encode(&self, version: &Version) -> Vec<u8> {
        let mut data = Vec::new();
        if let Some(graphic_control_extension) = &self.graphic_control_extension {
            if *version == Version::V87a {
                info!("Discarding graphic control extension because it's not supported in the target version '{:?}'", version);
            } else {
                data.extend_from_slice(&graphic_control_extension.encode());
            }
        }
        data.push(Self::SEPARATOR);
        data.push((self.image_left_position >> 8) as u8);
        data.push(((self.image_left_position << 8) >> 8) as u8);
        data.push((self.image_top_position >> 8) as u8);
        data.push(((self.image_top_position << 8) >> 8) as u8);
        data.push((self.image_width >> 8) as u8);
        data.push(((self.image_width << 8) >> 8) as u8);
        data.push((self.image_height >> 8) as u8);
        data.push(((self.image_height << 8) >> 8) as u8);
        data.push(self.packed_fields);
        if let Some(local_color_table) = &self.local_color_table {
            data.extend_from_slice(local_color_table.pixels);
        }
        data.extend_from_slice(&self.image_data.encode());
        data
    }
}

impl<'a> TableBasedImageData<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.lzw_minimum_code_size);
        data.extend_from_slice(&self.image_data.encode());
        data
    }
}
