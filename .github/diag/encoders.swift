import AVFoundation
import VideoToolbox
import Foundation

func makeAudio(_ ptsUs: Int64, frames: Int) -> CMSampleBuffer {
  var asbd = AudioStreamBasicDescription(mSampleRate: 48000, mFormatID: kAudioFormatLinearPCM, mFormatFlags: kAudioFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked, mBytesPerPacket: 4, mFramesPerPacket: 1, mBytesPerFrame: 4, mChannelsPerFrame: 2, mBitsPerChannel: 16, mReserved: 0)
  var format: CMAudioFormatDescription?
  CMAudioFormatDescriptionCreate(allocator: nil, asbd: &asbd, layoutSize: 0, layout: nil, magicCookieSize: 0, magicCookie: nil, extensions: nil, formatDescriptionOut: &format)
  let n = frames * 4
  var block: CMBlockBuffer?
  CMBlockBufferCreateWithMemoryBlock(allocator: nil, memoryBlock: nil, blockLength: n, blockAllocator: nil, customBlockSource: nil, offsetToData: 0, dataLength: n, flags: 0, blockBufferOut: &block)
  CMBlockBufferFillDataBytes(with: 0, blockBuffer: block!, offsetIntoDestination: 0, dataLength: n)
  var timing = CMSampleTimingInfo(duration: CMTime(value: 1, timescale: 48000), presentationTimeStamp: CMTime(value: ptsUs, timescale: 1_000_000), decodeTimeStamp: .invalid)
  var sample: CMSampleBuffer?
  CMSampleBufferCreateReady(allocator: nil, dataBuffer: block, formatDescription: format, sampleCount: frames, sampleTimingEntryCount: 1, sampleTimingArray: &timing, sampleSizeEntryCount: 0, sampleSizeArray: nil, sampleBufferOut: &sample)
  return sample!
}

func wait(_ i: AVAssetWriterInput, _ w: AVAssetWriter) -> Bool {
  let t = Date()
  while !i.isReadyForMoreMediaData && w.status == .writing && Date().timeIntervalSince(t) < 5 { Thread.sleep(forTimeInterval: 0.01) }
  return i.isReadyForMoreMediaData
}

func run(audio: Bool, software: Bool, colour: Bool) {
  let url = URL(fileURLWithPath: NSTemporaryDirectory() + UUID().uuidString + ".mp4")
  let w = try! AVAssetWriter(outputURL: url, fileType: .mp4)
  var vs: [String: Any] = [AVVideoCodecKey: AVVideoCodecType.h264, AVVideoWidthKey: 320, AVVideoHeightKey: 180,
    AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 2_000_000, AVVideoProfileLevelKey: AVVideoProfileLevelH264MainAutoLevel]]
  if colour { vs[AVVideoColorPropertiesKey] = [AVVideoColorPrimariesKey: AVVideoColorPrimaries_ITU_R_709_2, AVVideoTransferFunctionKey: AVVideoTransferFunction_ITU_R_709_2, AVVideoYCbCrMatrixKey: AVVideoYCbCrMatrix_ITU_R_709_2] }
  if software { vs[AVVideoEncoderSpecificationKey] = [kVTVideoEncoderSpecification_EnableHardwareAcceleratedVideoEncoder as String: false] }
  let v = AVAssetWriterInput(mediaType: .video, outputSettings: vs)
  v.expectsMediaDataInRealTime = false
  let ad = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: v, sourcePixelBufferAttributes: [kCVPixelBufferPixelFormatTypeKey as String: Int(kCVPixelFormatType_32BGRA), kCVPixelBufferWidthKey as String: 320, kCVPixelBufferHeightKey as String: 180])
  w.add(v)
  var a: AVAssetWriterInput?
  if audio {
    let ai = AVAssetWriterInput(mediaType: .audio, outputSettings: [AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 48000, AVNumberOfChannelsKey: 2, AVEncoderBitRateKey: 96000])
    ai.expectsMediaDataInRealTime = false; w.add(ai); a = ai
  }
  w.startWriting(); w.startSession(atSourceTime: .zero)
  var written = 0
  var af: Int64 = 0
  for i in 0..<39 {
    var buf: CVPixelBuffer?
    CVPixelBufferPoolCreatePixelBuffer(nil, ad.pixelBufferPool!, &buf)
    guard wait(v, w) else { print("  video stalled at frame \(i), status \(w.status.rawValue) \(String(describing: w.error))"); break }
    if !ad.append(buf!, withPresentationTime: CMTime(value: Int64(i) * 1_000_000 / 30, timescale: 1_000_000)) { print("  append failed \(i) \(String(describing: w.error))"); break }
    written += 1
    if let a {
      let end = Int64(i + 1) * 48000 / 30
      while af < end {
        let n = Int(min(1024, end - af))
        guard wait(a, w) else { print("  audio stalled at \(af)"); break }
        a.append(makeAudio(af * 1_000_000 / 48000, frames: n)); af += Int64(n)
      }
    }
  }
  w.cancelWriting()
  print("audio=\(audio) software=\(software) colour=\(colour): wrote \(written)/39")
}

run(audio: false, software: false, colour: true)
run(audio: true, software: false, colour: true)
run(audio: true, software: true, colour: true)
run(audio: true, software: false, colour: false)
