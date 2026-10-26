"""Validated raster crop; zero is a real coordinate, never a centering default."""
from pathlib import Path
import hashlib,subprocess

def crop_exact(source,destination,x,y,width,height):
    swift=Path(__file__).with_name('strict_crop.swift')
    tag=hashlib.sha256(swift.read_bytes()).hexdigest()[:12]
    binary=Path('/tmp')/('tartarus-strict-crop-'+tag)
    if not binary.exists():
        subprocess.run(['swiftc','-module-cache-path','/tmp/tartarus-swift-module-cache',str(swift),'-o',str(binary)],check=True,capture_output=True)
    subprocess.run([str(binary),'crop',str(source),str(destination),str(x),str(y),str(width),str(height),'raw-top-left'],check=True,capture_output=True)

if __name__=='__main__':
    import sys
    crop_exact(sys.argv[1],sys.argv[2],*[int(v) for v in sys.argv[3:7]])
