import AppKit
import Foundation
import CoreText
let root=URL(fileURLWithPath:CommandLine.arguments[1])
let bitmap=NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:1000,pixelsHigh:370,bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,bytesPerRow:0,bitsPerPixel:0)!
NSGraphicsContext.saveGraphicsState();NSGraphicsContext.current=NSGraphicsContext(bitmapImageRep:bitmap)
NSColor.white.setFill();NSRect(x:0,y:0,width:1000,height:370).fill()
let fontURL=URL(fileURLWithPath:"/System/Library/Fonts/Supplemental/NotoSansCuneiform-Regular.ttf")
CTFontManagerRegisterFontsForURL(fontURL as CFURL,.process,nil)
let descriptors=CTFontManagerCreateFontDescriptorsFromURL(fontURL as CFURL) as! [CTFontDescriptor]
let font=CTFontCreateWithFontDescriptor(descriptors[0],170,nil) as NSFont
for (i,item) in [("GI","𒄀"),("ZI","𒍣"),("IN","𒅔")].enumerated(){
 (item.1 as NSString).draw(at:NSPoint(x:i*330+20,y:65),withAttributes:[.font:font,.foregroundColor:NSColor.black])
 (item.0 as NSString).draw(at:NSPoint(x:i*330+20,y:20),withAttributes:[.font:NSFont.systemFont(ofSize:27),.foregroundColor:NSColor.black])
}
("MODERN FONT REFERENCE — NOT AN ANCIENT SIGN COPY" as NSString).draw(at:NSPoint(x:20,y:330),withAttributes:[.font:NSFont.systemFont(ofSize:22),.foregroundColor:NSColor.black])
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("round35_sources/modern_glyph_reference.png"))
