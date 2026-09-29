#!/usr/bin/env python3
from pathlib import Path
import sys
from PIL import Image, ImageDraw, ImageFont
root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=True)
font=ImageFont.load_default(size=64)
image=Image.new('RGB',(760,200),'white')
ImageDraw.Draw(image).text((40,55),'HELLO WORLD',fill='black',font=font)
image.save(root/'ocr-fixture.png')
