use std::convert::TryInto;

use resource_info_table::{resource_info, ResourceInfo};
use texpresso::Format;

// 0 - Power
// 1 - Gravity
// 2 - Varia
// 3 - Phazon

pub const POWER_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/powersuit_high_rez_bound.CMDL
    resource_info!("power_head_chest.TXTR"),
    resource_info!("power_head_chest_incan.TXTR"),
    resource_info!("power_arms.TXTR"),
    resource_info!("power_arms_incan.TXTR"),
    resource_info!("power_torso.TXTR"),
    resource_info!("power_legs.TXTR"),
    // Low poly TestAnim/PowerSuit.CMDL
    resource_info!("5C02CF66.TXTR"),
    resource_info!("6FC1D36D.TXTR"),
    // Left arm SamusGun/LeftArm.CMDL
    resource_info!("B2F8703C.TXTR"),
    resource_info!("1AE46C50.TXTR"),
    // Morph ball - TestAnim/SamusBallNew.CMDL MS 0
    resource_info!("C01FFF01.TXTR"),
    // MB Low poly - TestAnim/SamusBallLowPolyCMDL.CMDL MS 0
    // resource_info!("C01FFF01.TXTR"),

    // No spider ball... yet
];

pub const VARIA_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/variasuit_high_rez_bound.CMDL
    resource_info!("gravity_head_chest.TXTR"),
    resource_info!("power_head_chest_incan.TXTR"),
    resource_info!("gravity_torso_ball.TXTR"),
    resource_info!("gravity_legs.TXTR"),
    resource_info!("gravity_legs_incan.TXTR"),
    resource_info!("gravity_arms.TXTR"),
    resource_info!("gravity_arms_incan.TXTR"),
    // Low res? TestAnim/VariaSuit.CMDL
    resource_info!("D2149656.TXTR"),
    resource_info!("C06147B3.TXTR"),
    // Left hand - SamusGun/Varia.CMDL
    resource_info!("309AA3D4.TXTR"),
    resource_info!("5D380050.TXTR"),
    // Morph ball - TestAnim/SamusBallNew.CMDL MS 1
    resource_info!("49A1D81D.TXTR"),
    // MB Low poly - TestAnim/SamusBallLowPolyCMDL.CMDL MS 1
    // resource_info!("49A1D81D.TXTR"),

    // Spider ball - Uncategorized/spiderball_gravity.CMDL MS 1
    resource_info!("2EE6F56F.TXTR"),
    resource_info!("AD3748D3.TXTR"),
    // Spider ball glass - TestAnim/SamusSpiderBallGlassCMDL.CMDL - MS 1
    resource_info!("9024CB39.TXTR"),
    resource_info!("7A755049.TXTR"),
    // SB Low poly - TestAnim/SamusSpiderBallLowPolyCMDL.CMDL MS 1
    resource_info!("07675658.TXTR"),
];

pub const GRAVITY_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/gravitysuit_high_rez_bound.CMDL
    resource_info!("f_varia_legs.TXTR"),
    resource_info!("f_varia_legs_incan.TXTR"),
    resource_info!("spider_glass.TXTR"),
    resource_info!("f_varia_torso.TXTR"),
    resource_info!("f_varia_head_chest.TXTR"),
    resource_info!("f_varia_head_incan.TXTR"),
    resource_info!("f_varia_arms.TXTR"),
    resource_info!("f_varia_arms_incan.TXTR"),
    // Low poly TestAnim/GravitySuit.CMDL
    resource_info!("349AD971.TXTR"),
    resource_info!("A082D0BF.TXTR"),
    resource_info!("648AF351.TXTR"),
    resource_info!("5B05039F.TXTR"),
    resource_info!("1C38E5E2.TXTR"),
    // Left arm SamusGun/Gravity.CMDL
    resource_info!("985C0EAA.TXTR"),
    resource_info!("60EA8AC4.TXTR"),
    // Doesn't have a morph-ball model

    // Spider ball - TestAnim/Node1_1.CMDL MS 0
    resource_info!("50A70472.TXTR"),
    resource_info!("1AEC5A79.TXTR"),
    // Spider ball glass - TestAnim/SamusSpiderBallGlassCMDL.CMDL - MS 0
    resource_info!("27FFD993.TXTR"),
    // SB Low poly - TestAnim/SamusSpiderBallLowPolyCMDL.CMDL MS 0
    resource_info!("BA7DF5D6.TXTR"),
];

pub const PHAZON_SUIT_TEXTURES: &[ResourceInfo] = &[
    // high res Characters/Samus/cooked/phazon_suit_high_rez_bound.CMDL
    resource_info!("phason_arm_black.TXTR"),
    resource_info!("phason_arm_incandescence.TXTR"),
    resource_info!("phason_head_black.TXTR"),
    resource_info!("phason_head_incandescence.TXTR"),
    resource_info!("phason_legs_black.TXTR"),
    resource_info!("phason_legs_incandescence.TXTR"),
    resource_info!("phason_torso_black.TXTR"),
    resource_info!("phason_torso_specialincandescence.TXTR"),
    resource_info!("phasonred_rampincandes.TXTR"),
    resource_info!("Characters/common_textures/glow10.TXTR"),
    // Low poly TestAnim/PhazonSuit.CMDL
    resource_info!("08FA7447.TXTR"),
    resource_info!("EC4184DF.TXTR"),
    // Left hand SamusGun/Phazon.CMDL
    resource_info!("C94DD270.TXTR"),
    resource_info!("1A9153A8.TXTR"),
    // resource_info!("1A9153A8.TXTR"),

    // Spider ball TestAnim/Node1_0.CMDL
    resource_info!("8B105F2E.TXTR"),
    resource_info!("2F1AC0DD.TXTR"),
    // resource_info!("Uncategorized/8B105F2E.TXTR"),
    resource_info!("8BF681E5.TXTR"),
    resource_info!("51F20A44.TXTR"),
    //
    // Glass ball TestAnim/SamusPhazonBallGlassCMDL.CMDL
    resource_info!("D3889172.TXTR"),
    resource_info!("0B3DBDB4.TXTR"),
    // resource_info!("Uncategorized/596C7FFF.TXTR"),

    // Spider ball Low poly - TestAnim/SamusSpiderBallLowPolyCMDL.CMDL MS 2
    resource_info!("06CE2C16.TXTR"),
];

// Fusion morph ball ANCS TestAnim/Fusion_Ball.ANCS
pub const FUSION_POWER_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/fusion_suit_high_rez_bound.CMDL
    // Low poly TestAnim/FusionSuit.CMDL
    // Left arm SamusGun/Fusion.CMDL
];

pub const FUSION_VARIA_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/fusion_varia_high_rez_bound.CMDL
    // Low poly TestAnim/Fusion_Varia.CMDL
    // Left arm SamusGun/FusionV.CMDL
];

pub const FUSION_GRAVITY_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/fusion_gravity_high_rez_bound.CMDL
    // Low poly TestAnim/Fusion_Gravity.CMDL
    // Left arm SamusGun/FusionG.CMDL
];

pub const FUSION_PHAZON_SUIT_TEXTURES: &[ResourceInfo] = &[
    // High res Characters/Samus/cooked/fusion_phazon_high_rez_bound.CMDL
    // Low poly TestAnim/Fusion_Phazon.CMDL
    // Left arm SamusGun/FusionP.CMDL
];

struct CmprPixelIter {
    cnt: usize,
    width: usize,
    height: usize,
}

impl CmprPixelIter {
    fn new(width: usize, height: usize) -> Self {
        CmprPixelIter {
            cnt: 0,
            width,
            height,
        }
    }
}

impl Iterator for CmprPixelIter {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let inner_x = self.cnt & 1;
        let inner_y = (self.cnt & 2) >> 1;
        let block_x = ((self.cnt & !3) >> 2) % (self.width / 8);
        let block_y = ((self.cnt & !3) >> 2) / (self.width / 8);

        let first_pixel_x = block_x * 8 + inner_x * 4;
        // TODO: Check for underflow, if we did, return None?
        let first_pixel_y = self.height - 4 - (block_y * 8 + inner_y * 4);
        self.cnt += 1;
        Some((first_pixel_x, first_pixel_y))
    }
}

pub fn cmpr_decompress(compressed: &[u8], width: usize, height: usize, decompressed: &mut [u8]) {
    let cmpr_iter = CmprPixelIter::new(width, height);
    for (chunk, (first_pixel_x, first_pixel_y)) in compressed.chunks_exact(8).zip(cmpr_iter) {
        let decompressed_pixels = decompress_dxt1gcn_block(chunk);
        for y in 0..4 {
            for x in 0..4 {
                let pixel_x = first_pixel_x + x;
                let pixel_y = first_pixel_y + (3 - y);
                let pixel_start = (pixel_y * width + pixel_x) * 4;
                let pixel_data = decompressed_pixels[y * 4 + x];
                decompressed[pixel_start..pixel_start + 4].copy_from_slice(&pixel_data);
            }
        }
    }
}

pub fn cmpr_compress(uncompressed: &[u8], width: usize, height: usize, compressed: &mut [u8]) {
    let cmpr_iter = CmprPixelIter::new(width, height);
    for (chunk, (first_pixel_x, first_pixel_y)) in compressed.chunks_exact_mut(8).zip(cmpr_iter) {
        let mut uncompressed_pixels = [[0u8; 4]; 16];

        for y in 0..4 {
            for x in 0..4 {
                let pixel_x = first_pixel_x + x;
                let pixel_y = first_pixel_y + (3 - y);
                let pixel_start = (pixel_y * width + pixel_x) * 4;
                uncompressed_pixels[y * 4 + x] = uncompressed[pixel_start..pixel_start + 4]
                    .try_into()
                    .unwrap();
            }
        }

        compress_dxt1gcn_block(uncompressed_pixels, chunk);
    }
}

pub fn huerotate_matrix(angle: f32) -> [f32; 9] {
    let cosv = (angle * std::f32::consts::PI / 180.0).cos();
    let sinv = (angle * std::f32::consts::PI / 180.0).sin();
    [
        // Reds
        0.213 + cosv * 0.787 - sinv * 0.213,
        0.715 - cosv * 0.715 - sinv * 0.715,
        0.072 - cosv * 0.072 + sinv * 0.928,
        // Greens
        0.213 - cosv * 0.213 + sinv * 0.143,
        0.715 + cosv * 0.285 + sinv * 0.140,
        0.072 - cosv * 0.072 - sinv * 0.283,
        // Blues
        0.213 - cosv * 0.213 - sinv * 0.787,
        0.715 - cosv * 0.715 + sinv * 0.715,
        0.072 + cosv * 0.928 + sinv * 0.072,
    ]
}

pub fn huerotate_color(matrix: [f32; 9], r: u8, g: u8, b: u8) -> [u8; 3] {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    [
        (matrix[0] * r + matrix[1] * g + matrix[2] * b).clamp(0.0, 255.0) as u8,
        (matrix[3] * r + matrix[4] * g + matrix[5] * b).clamp(0.0, 255.0) as u8,
        (matrix[6] * r + matrix[7] * g + matrix[8] * b).clamp(0.0, 255.0) as u8,
    ]
}

// Adapted from image-rs
pub fn huerotate_in_place(image: &mut [u8], width: usize, height: usize, matrix: [f32; 9]) {
    for y in 0..height {
        for x in 0..width {
            let start = (y * width + x) * 4;
            let pixel = &mut image[start..start + 4];

            let new_rgb = huerotate_color(matrix, pixel[0], pixel[1], pixel[2]);

            let outpixel = [new_rgb[0], new_rgb[1], new_rgb[2], pixel[3]];

            pixel.copy_from_slice(&outpixel[..]);
        }
    }
}

pub fn compress_dxt1gcn_block(rgba: [[u8; 4]; 16], block: &mut [u8]) {
    Format::Bc1.compress_block_masked(
        rgba,
        0xFFFF,
        texpresso::Params {
            algorithm: texpresso::Algorithm::IterativeClusterFit,
            ..Default::default()
        },
        block,
    );
    block.swap(0, 1);
    block.swap(2, 3);
    for byte in block[4..8].iter_mut() {
        *byte = reverse_byte(*byte);
    }
}

pub fn decompress_dxt1gcn_block(block: &[u8]) -> [[u8; 4]; 16] {
    let mut compressed = [0u8; 8];
    compressed[0] = block[1];
    compressed[1] = block[0];
    compressed[2] = block[3];
    compressed[3] = block[2];
    for (byte, val) in compressed[4..8].iter_mut().zip(block[4..8].iter()) {
        *byte = reverse_byte(*val);
    }
    Format::Bc1.decompress_block(&compressed)
}

fn reverse_byte(byte: u8) -> u8 {
    ((byte & 0b00000011) << 6)
        | ((byte & 0b00001100) << 2)
        | ((byte & 0b00110000) >> 2)
        | ((byte & 0b11000000) >> 6)
}
