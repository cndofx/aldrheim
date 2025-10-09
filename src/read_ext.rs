use byteorder::{LittleEndian, ReadBytesExt};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

pub trait MyReadBytesExt: ReadBytesExt {
    fn read_bool(&mut self) -> std::io::Result<bool>;
    fn read_7bit_encoded_i32(&mut self) -> std::io::Result<i32>;
    fn read_7bit_length_string(&mut self) -> std::io::Result<String>;
    fn read_vec2(&mut self) -> std::io::Result<Vec2>;
    fn read_vec3(&mut self) -> std::io::Result<Vec3>;
    fn read_vec4(&mut self) -> std::io::Result<Vec4>;
    fn read_mat4(&mut self) -> std::io::Result<Mat4>;
    fn read_quat(&mut self) -> std::io::Result<Quat>;
}

impl<R: ReadBytesExt> MyReadBytesExt for R {
    fn read_bool(&mut self) -> std::io::Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    fn read_7bit_encoded_i32(&mut self) -> std::io::Result<i32> {
        let mut result: i32 = 0;
        let mut bits_read = 0;

        loop {
            let byte = self.read_u8()? as i32;
            result |= (byte & 0x7f) << bits_read;
            bits_read += 7;

            if byte & 0x80 == 0 {
                break;
            }
        }

        Ok(result)
    }

    fn read_7bit_length_string(&mut self) -> std::io::Result<String> {
        let len = self.read_7bit_encoded_i32()? as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(self.read_u8()? as char);
        }
        Ok(s)
    }

    fn read_vec2(&mut self) -> std::io::Result<Vec2> {
        let x = self.read_f32::<LittleEndian>()?;
        let y = self.read_f32::<LittleEndian>()?;
        Ok(Vec2::new(x, y))
    }

    fn read_vec3(&mut self) -> std::io::Result<Vec3> {
        let x = self.read_f32::<LittleEndian>()?;
        let y = self.read_f32::<LittleEndian>()?;
        let z = self.read_f32::<LittleEndian>()?;
        Ok(Vec3::new(x, y, z))
    }

    fn read_vec4(&mut self) -> std::io::Result<Vec4> {
        let x = self.read_f32::<LittleEndian>()?;
        let y = self.read_f32::<LittleEndian>()?;
        let z = self.read_f32::<LittleEndian>()?;
        let w = self.read_f32::<LittleEndian>()?;
        Ok(Vec4::new(x, y, z, w))
    }

    fn read_mat4(&mut self) -> std::io::Result<Mat4> {
        let m11 = self.read_f32::<LittleEndian>()?;
        let m12 = self.read_f32::<LittleEndian>()?;
        let m13 = self.read_f32::<LittleEndian>()?;
        let m14 = self.read_f32::<LittleEndian>()?;

        let m21 = self.read_f32::<LittleEndian>()?;
        let m22 = self.read_f32::<LittleEndian>()?;
        let m23 = self.read_f32::<LittleEndian>()?;
        let m24 = self.read_f32::<LittleEndian>()?;

        let m31 = self.read_f32::<LittleEndian>()?;
        let m32 = self.read_f32::<LittleEndian>()?;
        let m33 = self.read_f32::<LittleEndian>()?;
        let m34 = self.read_f32::<LittleEndian>()?;

        let m41 = self.read_f32::<LittleEndian>()?;
        let m42 = self.read_f32::<LittleEndian>()?;
        let m43 = self.read_f32::<LittleEndian>()?;
        let m44 = self.read_f32::<LittleEndian>()?;

        let mat = Mat4::from_cols_array(&[
            m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44,
        ])
        .transpose();

        Ok(mat)
    }

    fn read_quat(&mut self) -> std::io::Result<Quat> {
        let x = self.read_f32::<LittleEndian>()?;
        let y = self.read_f32::<LittleEndian>()?;
        let z = self.read_f32::<LittleEndian>()?;
        let w = self.read_f32::<LittleEndian>()?;
        Ok(Quat::from_xyzw(x, y, z, w))
    }
}
