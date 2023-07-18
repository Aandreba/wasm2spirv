#include <simd/simd.h>

#include <metal_stdlib>

using namespace metal;

struct _9 {
    uint _m0;
};

struct _13 {
    float _m0;
};

struct _17 {
    float _m0[1];
};

kernel void saxpy(device void* spvBufferAliasSet0Binding1 [[buffer(0)]], device _17& _19 [[buffer(1)]], uint3 gl_GlobalInvocationID [[thread_position_in_grid]], uint3 gl_NumWorkGroups [[threadgroups_per_grid]]) {
    device auto& _11 = *(device _9*)spvBufferAliasSet0Binding1;
    device auto& _15 = *(device _13*)spvBufferAliasSet0Binding1;
    device auto& _20 = *(device _17*)spvBufferAliasSet0Binding1;
    ulong _54 = 0ul;
    ulong _33 = ulong(gl_GlobalInvocationID.x);
    ulong _35 = _33 << 2ul;
    ulong _38 = ulong(gl_NumWorkGroups.x);
    ulong _39 = _38 << 2ul;
    ulong _28 = ulong(_11._m0);
    device _17* _30;
    for (ulong _24 = _33, _25 = _35, _26 = _38, _27 = _39; !(_24 >= _28);) {
        _30 = &_20;
        _54 = _25;
        _20._m0[_25 / 4ul] = _30->_m0[_54 / 4ul] + (_19._m0[_25 / 4ul] * _15._m0);
        _25 += _27;
        _24 += _26;
        continue;
    }
}
