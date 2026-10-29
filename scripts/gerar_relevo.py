import sys, numpy as np
from PIL import Image
Image.MAX_IMAGE_PIXELS=None
src=np.asarray(Image.open(sys.argv[1]).convert('L'),dtype=np.float32)
H,W=src.shape; w,h=5632,2048
ymax=1.25*np.log(np.tan(np.pi/4+0.4*np.radians(float(sys.argv[3]))))  # MILLER_Y_MAX
py=(np.arange(h)+0.5)/h
ym=ymax-py*2*ymax
phi=(np.arctan(np.exp(ym/1.25))-np.pi/4)/0.4
lat=np.degrees(phi)
sy=np.clip(((90-lat)/180*H).astype(int),0,H-1)
sx=np.clip((((np.arange(w)+0.5)/w)*W).astype(int),0,W-1)
out=src[sy][:,sx]
Image.fromarray(out.astype(np.uint8),'L').save(sys.argv[2])
print(out.min(),out.mean(),out.max())
