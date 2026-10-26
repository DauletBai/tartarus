// Scientific image extraction. Coordinates are raw raster pixels from top left.
import Foundation
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers

let a=CommandLine.arguments
guard a.count==9, a[1]=="crop", let x=Int(a[4]), let y=Int(a[5]), let w=Int(a[6]), let h=Int(a[7]), a[8]=="raw-top-left" else {
    fatalError("usage: strict_crop crop INPUT OUTPUT X Y WIDTH HEIGHT raw-top-left")
}
let srcURL=URL(fileURLWithPath:a[2]), dstURL=URL(fileURLWithPath:a[3])
guard let src=CGImageSourceCreateWithURL(srcURL as CFURL,nil), let im=CGImageSourceCreateImageAtIndex(src,0,nil) else {fatalError("Cannot decode source")}
let props=CGImageSourceCopyPropertiesAtIndex(src,0,nil) as? [CFString:Any]
let orientation=(props?[kCGImagePropertyOrientation] as? NSNumber)?.intValue ?? 1
guard orientation==1 else {fatalError("Non-upright EXIF orientation; normalize explicitly before assigning coordinates")}
guard x>=0,y>=0,w>0,h>0,x+w<=im.width,y+h<=im.height else {fatalError("Out of source bounds")}
guard let crop=im.cropping(to:CGRect(x:x,y:y,width:w,height:h)) else {fatalError("Crop failed")}
let jpg=["jpg","jpeg"].contains(dstURL.pathExtension.lowercased())
let uti=(jpg ? UTType.jpeg.identifier : UTType.png.identifier) as CFString
guard let dst=CGImageDestinationCreateWithURL(dstURL as CFURL,uti,1,nil) else {fatalError("Cannot create destination")}
CGImageDestinationAddImage(dst,crop,jpg ? [kCGImageDestinationLossyCompressionQuality:1.0] as CFDictionary : nil)
guard CGImageDestinationFinalize(dst) else {fatalError("Encoding failed")}
print("\(im.width)x\(im.height) → x\(x) y\(y) w\(w) h\(h); origin=raw-top-left; orientation=\(orientation)")
