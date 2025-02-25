#pragma endian big

struct Header {
    u16 size_x ;
    u16 size_y;
    u16 size_z;
    u8 blocks[size_x*size_y*size_z];
};



Header Data @ 0x00;