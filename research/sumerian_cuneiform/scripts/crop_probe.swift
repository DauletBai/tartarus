import Foundation
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
let a=CommandLine.arguments
if a[1]=="fixture" {
    let w=120,h=100
    var pixels=[UInt8](repeating:255,count:w*h*4)
    for y in 0..<h {for x in 0..<w {let i=(y*w+x)*4;pixels[i]=UInt8(x);pixels[i+1]=UInt8(y);pixels[i+2]=UInt8((x+y)%256)}}
    let data=Data(pixels) as CFData
    let im=CGImage(width:w,height:h,bitsPerComponent:8,bitsPerPixel:32,bytesPerRow:w*4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGBitmapInfo(rawValue:CGImageAlphaInfo.last.rawValue),provider:CGDataProvider(data:data)!,decode:nil,shouldInterpolate:false,intent:.defaultIntent)!
    let dst=CGImageDestinationCreateWithURL(URL(fileURLWithPath:a[2]) as CFURL,UTType.png.identifier as CFString,1,nil)!
    CGImageDestinationAddImage(dst,im,nil);guard CGImageDestinationFinalize(dst) else {fatalError("write")}
} else {
    let src=CGImageSourceCreateWithURL(URL(fileURLWithPath:a[2]) as CFURL,nil)!
    let im=CGImageSourceCreateImageAtIndex(src,0,nil)!
    var samples=[[UInt8]]()
    for (x,y) in [(0,0),(im.width-1,0),(0,im.height-1),(im.width-1,im.height-1)] {
        let pixel=im.cropping(to:CGRect(x:x,y:y,width:1,height:1))!
        var data=[UInt8](repeating:0,count:4)
        data.withUnsafeMutableBytes {buf in
            let ctx=CGContext(data:buf.baseAddress,width:1,height:1,bitsPerComponent:8,bytesPerRow:4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
            ctx.draw(pixel,in:CGRect(x:0,y:0,width:1,height:1))
        };samples.append(data)
    }
    let obj:[String:Any]=["width":im.width,"height":im.height,"corners_RGBA":samples]
    let d=try JSONSerialization.data(withJSONObject:obj,options:[.sortedKeys]);print(String(data:d,encoding:.utf8)!)
}
