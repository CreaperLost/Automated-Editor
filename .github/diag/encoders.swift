import VideoToolbox
import Foundation
var list: CFArray?
VTCopyVideoEncoderList(nil, &list)
for case let e as [String: Any] in (list as? [Any]) ?? [] {
  print(e[kVTVideoEncoderList_EncoderID as String] ?? "?", e[kVTVideoEncoderList_IsHardwareAccelerated as String] ?? "-")
}
var session: VTCompressionSession?
let st = VTCompressionSessionCreate(allocator: nil, width: 320, height: 180, codecType: kCMVideoCodecType_H264, encoderSpecification: nil, imageBufferAttributes: nil, compressedDataAllocator: nil, outputCallback: nil, refcon: nil, compressionSessionOut: &session)
print("VTCompressionSessionCreate h264 320x180:", st)
