import AppKit
import Foundation
let root=URL(fileURLWithPath:CommandLine.arguments[1])
let input=root.appendingPathComponent("round33_sources/contact_input.json")
let items=try JSONSerialization.jsonObject(with:Data(contentsOf:input)) as! [[String:String]]
let cols=4, rows=4, cellW=400, cellH=480
let width=cols*cellW, height=rows*cellH
for page in 0..<((items.count+15)/16) {
 let bitmap=NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:width,pixelsHigh:height,bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,bytesPerRow:0,bitsPerPixel:0)!
 NSGraphicsContext.saveGraphicsState()
 NSGraphicsContext.current=NSGraphicsContext(bitmapImageRep:bitmap)
 NSColor.white.setFill(); NSRect(x:0,y:0,width:width,height:height).fill()
 for idx in 0..<16 {
  let n=page*16+idx; if n>=items.count {break}
  let item=items[n];let col=idx%cols,row=idx/cols
  let x=CGFloat(col*cellW),y=CGFloat(height-(row+1)*cellH)
  let target=NSRect(x:x+8,y:y+40,width:CGFloat(cellW-16),height:CGFloat(cellH-48))
  if let im=NSImage(contentsOfFile:item["path"]!) {
   let size=im.size;let ratio=min(target.width/size.width,target.height/size.height)
   let w=size.width*ratio,h=size.height*ratio
   im.draw(in:NSRect(x:target.midX-w/2,y:target.midY-h/2,width:w,height:h),from:.zero,operation:.sourceOver,fraction:1)
  }
  let attrs:[NSAttributedString.Key:Any]=[.font:NSFont.systemFont(ofSize:13),.foregroundColor:NSColor.black]
  let label=item["id"]!+"  "+item["label"]!
  (label as NSString).draw(in:NSRect(x:x+8,y:y+3,width:CGFloat(cellW-16),height:35),withAttributes:attrs)
 }
 NSGraphicsContext.restoreGraphicsState()
 let data=bitmap.representation(using:.jpeg,properties:[.compressionFactor:0.88])!
 let file=root.appendingPathComponent(String(format:"round33_sources/contact_%02d.jpg",page+1))
 try data.write(to:file)
 print(file.lastPathComponent)
}
