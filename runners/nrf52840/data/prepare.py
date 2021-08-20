#!/usr/bin/python3

import os
import subprocess
import sys

def get_img_dim(fn):
	p = subprocess.Popen("identify %s " % fn, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, shell=True, text=True)
	pout, perr = p.communicate()
	assert perr is None
	assert p.returncode == 0
	dim = pout.strip().split(" ")[2].split("x")
	return int(dim[0]), int(dim[1])

def png_to_raw565(fn, cw, ch, cntx, cnty):
	iw, ih = get_img_dim(fn)
	assert iw == cw*cntx
	assert ih == ch*cnty
	fnbmp = fn.replace(".png", ".bmp")
	fnraw = fn.replace(".png", ".raw")

	p = subprocess.Popen("convert %s -type TrueColor -define BMP:subtype=rgb565 %s" % (fn, fnbmp),
			stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
			shell=True)
	p.communicate()
	assert p.returncode == 0

	fo = open(fnraw, "wb")
	with open(fnbmp, "rb") as fb:
		fb.seek(138)
		bmpdata = fb.read()
	assert len(bmpdata) == iw*ih*2

	for i in range(cntx*cnty):
		charposx = i % cntx
		charposy = i // cntx
		bytepos = (ih-1-(charposy*ch))*iw*2 + (charposx*cw)*2
		for y in range(ch):
			rowpos = bytepos - y*2*iw
			fo.write(bmpdata[rowpos:rowpos+2*cw])

	fo.close()
	os.unlink(fnbmp)

try:
	png_to_raw565("font_9x18.png", 9, 18, 16, 12)
	png_to_raw565("font_9x18_bold.png", 9, 18, 16, 12)
except FileNotFoundError:
	print("ERROR: font files from embedded-graphics crate needed.")
	print("Please copy or symlink fonts/png/font_9x18*.png here.")
	sys.exit(1)

sys.exit(0)
