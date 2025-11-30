use std::{f64::consts::PI, ops::Mul, sync::Mutex};
use serde_derive::{Deserialize, Serialize};
#[derive(Copy, Clone, PartialEq,Debug)]
#[derive(Serialize, Deserialize)]
pub struct Vec2r {
    pub x: f32,
    pub y: f32,
}
impl Vec2r{
    pub fn new(x: f32, y: f32) -> Self {
            Self { x, y }
    }
    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }
    pub fn dist(&self, other:Self)->f32{
        let p1 = self.x-other.x;
        let p2 = self.y-other.y;
        p1*p1+p2*p2
    }
    pub fn len_sqr(&self)->f32{
        self.x*self.x+self.y*self.y
    }
    pub fn len(&self)->f32{
        self.len_sqr().sqrt()
    }
    pub fn norm(&self)->Self{
        *self/self.len()
    }
    pub fn to_int(&self)->Vec2{
        Vec2{x:self.x as i32, y:self.y as i32}
    }
    pub fn angle(&self)->f32{
        f32::atan2(self.y, self.x)
    }
    pub fn from_angle(theta:f32)->Self{
        Self { x: theta.cos(), y: theta.sin() }
    }
    pub fn rotate(&self, dtheta:f32)->Self{
        let a = self.angle()+dtheta;
        let l = self.len();
        Self::from_angle(a)*l
    }

}
impl std::ops::Add for Vec2r{
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl std::ops::Sub for Vec2r {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl std::ops::AddAssign for Vec2r {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}
impl std::ops::SubAssign for Vec2r {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}
impl std::ops::Mul<f32> for Vec2r {
    type Output = Self;
    fn mul(self, v: f32) -> Self::Output {
        Self {
            x: self.x * v,
            y: self.y * v,
        }
    }
}
impl std::ops::Div<f32> for Vec2r {
    type Output = Self;
    fn div(self, v: f32) -> Self::Output {
        Self {
            x: self.x / v,
            y: self.y / v,
        }
    }
}
impl std::ops::MulAssign<f32> for Vec2r {
    fn mul_assign(&mut self, v: f32) {
        self.x *= v;
        self.y *= v;
    }
}
impl std::ops::DivAssign<f32> for Vec2r{
    fn div_assign(&mut self, v: f32) {
        self.x /= v;
        self.y /= v;
    }
}


#[derive(Serialize, Deserialize)]
#[derive(Copy, Clone, PartialEq, Eq,Debug)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}
impl Vec2 {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn dot(&self, other: Self) -> i32 {
        self.x * other.x + self.y * other.y
    }
    pub fn dist(&self, other:Self)->i32{
        let p1 = self.x-other.x;
        let p2 = self.y-other.y;
        p1*p1+p2*p2
    }
    pub fn len_sqr(&self)->i32{
        self.x*self.x+self.y*self.y
    }
    pub fn len(&self)->i32{
        self.len_sqr().isqrt()
    }
    pub fn norm(&self)->Self{
        *self/self.len()
    }
    pub fn to_real(&self)->Vec2r{
        Vec2r{x:self.x as f32, y:self.y as f32}
    }
}
impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}
impl std::ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
    }
}
impl std::ops::Mul<i32> for Vec2 {
    type Output = Self;
    fn mul(self, v: i32) -> Self::Output {
        Self {
            x: self.x * v,
            y: self.y * v,
        }
    }
}
impl std::ops::Div<i32> for Vec2 {
    type Output = Self;
    fn div(self, v: i32) -> Self::Output {
        Self {
            x: self.x / v,
            y: self.y / v,
        }
    }
}
impl std::ops::Rem<i32> for Vec2 {
    type Output = Self;
    fn rem(self, v: i32) -> Self::Output {
        Self {
            x: self.x % v,
            y: self.y % v,
        }
    }
}
impl std::ops::MulAssign<i32> for Vec2 {
    fn mul_assign(&mut self, v: i32) {
        self.x *= v;
        self.y *= v;
    }
}
impl std::ops::DivAssign<i32> for Vec2 {
    fn div_assign(&mut self, v: i32) {
        self.x /= v;
        self.y /= v;
    }
}
impl std::ops::RemAssign<i32> for Vec2 {
    fn rem_assign(&mut self, v: i32) {
        self.x %= v;
        self.y %= v;
    }
}
#[derive(Clone, Copy,Debug)]
pub struct BB {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl BB {
    pub fn intersects(&self, p: Vec2) -> bool {
        (p.x >= self.x && p.x < (self.x + self.w)) && (p.y >= self.y && p.y < (self.y + self.h))
    }
    pub fn from_points(p: &[Vec2]) -> Self {
        let mut min = p[0];
        let mut max = p[0];
        for i in p {
            if i.x > max.x {
                max.x = i.x;
            }
            if i.y > max.y {
                max.y = i.y;
            }
            if i.x < min.x {
                min.x = i.x
            }
            if i.y < min.y {
                min.y = i.y;
            }
        }
        if max.y == min.y {
            max.y+=1;
        }
        if max.x == min.x{
            max.x+=1;
        }
        Self {
            x: min.x,
            y: min.y,
            w: max.x - min.x,
            h: max.y - min.y,
        }
    }
}
//https://stackoverflow.com/questions/2049582/how-to-determine-if-a-point-is-in-a-2d-triangle
pub fn in_triangle(p1: Vec2, p2: Vec2, p3: Vec2, pos: Vec2) -> bool {
    fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> i32 {
        (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
    }
    let d1 = sign(pos, p1, p2);
    let d2 = sign(pos, p2, p3);
    let d3 = sign(pos, p3, p1);
    let hn = (d1 < 0) || (d2 < 0) || (d3 < 0);
    let hp = (d1 > 0) || (d2 > 0) || (d3 > 0);
    !(hn && hp)
}
pub fn triangle_area(p1:Vec2, p2:Vec2, p3:Vec2)->f64{
    let t1 = Vec3::new(p1.x as f64, p1.y as f64, 0.0);
    let t2 = Vec3::new(p2.x as f64, p2.y as f64, 0.0);
    let t3 = Vec3::new(p3.x as f64, p3.y as f64, 0.0);
    let v0 = t2-t1;
    let v1 = t3-t1;
    let c = v0.cross(v1);
    c.len()/2.0
}
#[derive(Copy, Clone, PartialEq,Debug)]
pub struct Vec3{
    pub x:f64, 
    pub y:f64, 
    pub z:f64
}
impl Vec3{
    pub fn new(x:f64, y:f64, z:f64)->Self{
        Self{x,y,z}
    }
    pub fn dot(&self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y+self.z*other.z
    }
    pub fn cross(&self, other:Self)->Vec3{
        let x = self.y*other.z-self.z*other.y;
        let y = self.z*other.x-self.x*other.z;
        let z = self.x*other.y-self.y*other.x;
        Self{x,y,z}
    }
    pub fn dist(&self, other:Self)->f64{
        let p1 = self.x-other.x;
        let p2 = self.y-other.y;
        let p3 = self.z-other.z;
        p1*p1+p2*p2+p3*p3
    }
    pub fn len_sqr(&self)->f64{
        self.x*self.x+self.y*self.y+self.z*self.z
    }
    pub fn len(&self)->f64{
        self.len_sqr().sqrt()
    }
    pub fn norm(&self)->Self{
        *self/self.len()
    }
    pub fn abs(&self)->Self{
        Self{x:self.x.abs(), y:self.y.abs(), z:self.z.abs()}
    }
}
impl std::ops::Add for Vec3{
    type Output = Self;
    fn add(self, other:Self)->Self::Output{
        Self{
            x:self.x+other.x,
            y:self.y+other.y,
            z:self.z+other.z,
        }
    }
}
impl std::ops::Sub for Vec3{
    type Output = Self;
    fn sub(self, other:Self)->Self::Output{
        Self{
            x:self.x-other.x,
            y:self.y-other.y,
            z:self.z-other.z,
        }
    }
}
impl std::ops::AddAssign for Vec3{
    fn add_assign(&mut self, other: Self) {
        self.x+= other.x;
        self.y+= other.y;
        self.z+= other.z;
    }
}
impl std::ops::SubAssign for Vec3{
    fn sub_assign(&mut self, other: Self) {
        self.x-= other.x;
        self.y-= other.y;
        self.z-= other.z;
    }
}
impl std::ops::Mul<f64> for Vec3{
    type Output = Self;
    fn mul(self, v:f64)->Self::Output{
        Self{
            x:self.x*v, 
            y:self.y*v, 
            z:self.z*v
        }
    }
}
impl std::ops::Div<f64> for Vec3{
    type Output = Self;
    fn div(self, v:f64)->Self::Output{
        Self{
            x:self.x/v, 
            y:self.y/v, 
            z:self.z/v
        }
    }
}
impl std::ops::DivAssign<f64> for Vec3{
    fn div_assign(&mut self, rhs: f64) {
        self.x/=rhs;
        self.y/= rhs;
        self.z/= rhs;
    }
}
impl std::ops::MulAssign<f64> for Vec3{
    fn mul_assign(&mut self, rhs: f64) {
        self.x*= rhs;
        self.y*= rhs;
        self.z*= rhs;
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Mat2{
    pub values:[[f64;2];2]
}
impl Mat2{
    pub fn new(x00:f64, x01:f64, x10:f64, x11:f64)->Self{
        Self { values:[[x00, x01],[x10, x11]] }
    }
    pub fn rotmat(rads:f64)->Self{
        Self{values:[[rads.cos(), -rads.sin()],[rads.sin(), rads.cos()]]}
    }
}
impl Mul<Vec2r> for Mat2{
    type Output = Vec2r;
    fn mul(self, rhs: Vec2r) -> Self::Output {
        let values = &self.values;
        let x =values[0][0] as f32*rhs.x+values[0][1]as f32*rhs.y;
        let y = values[1][0] as f32*rhs.x+values[1][1] as f32*rhs.y;
        Vec2r::new(x,y)
    }
}
static RAND_GEN:Mutex<u64> = Mutex::new(0);
pub fn srand_time(){
    let mut h = RAND_GEN.lock().unwrap();
    *h = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() %u64::MAX as u128)as u64;
    if *h == 0{
        *h += 13;
    }
}
pub fn srand(v:u64){
   let mut h = RAND_GEN.lock().unwrap();
   *h = v;
}
pub fn rand()->u64{
    let mut h = RAND_GEN.lock().unwrap();
    *h ^= h.wrapping_shl(13);
    *h ^= h.wrapping_shr(7);
    *h ^= h.wrapping_shl(17);
    *h
}
pub fn rand_int()->i64{
    unsafe{std::mem::transmute(rand())}
}
pub fn rand_float()->f64{
    const RES:u64 = 1_000_000;
    let base = ((rand()%RES)as f64)/(RES as f64);
    base
}
pub fn rand_angle()->f64{
    rand_float()*2.*PI
}
pub fn rand_vec2r()->Vec2r{
    let theta = rand_angle();
    Vec2r::new(theta.cos() as f32, theta.sin() as f32)
}
pub fn dist_to_line(start:Vec2r, end:Vec2r, p:Vec2r)->f32{
   /*let num = ((end.y-start.y)*p.x-(end.x-start.x)*p.y+end.x*start.y-end.y*start.x).abs();
    let denum = ((end.y-start.y)*(end.y-start.y)+(end.x-start.x)*(end.x-start.x)).sqrt();
    let out1 = num/denum;
    let out2 = end.dist(p);
    let out3 = start.dist(p);
    min(out1,min(out2, out3))*/
    let d = end-start;
    let dl = d.len();
    let dn = d/dl;
    let b = p-start;
    let dot = b.dot(dn);
    let proj = dn*b.dot(dn);
    let fproj = proj+start;
    if dot>dl{
        p.dist(end)
    }else if dot<0.0{
        p.dist(start)
    }
    else{
        fproj.dist(p)
    }

}
pub fn min<T:PartialOrd>(a:T, b:T)->T{
    if a<b{
        a
    }else{
        b
    }
}
pub fn max<T:PartialOrd>(a:T, b:T)->T{
    if a>b{
        a
    }else{
        b
    }
}