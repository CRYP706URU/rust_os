
# NOTE: This is actually done in Kernel/Makefile
logo.rs: TifflinLogoV1-128.png Makefile ConvertTo32bppRS
	./ConvertTo32bppRS $< $@ S_LOGO

power.r8: open-iconic_moon-2x.png Makefile
	convert $< -depth 8 -channel A -separate gray:.tmp.$(notdir $@)
	/bin/echo -ne "\x10\x00\x10\x00" | cat - .tmp.$(notdir $@) > $@

options.r8: open-iconic_cog-2x.png Makefile
	convert $< -depth 8 -channel A -separate gray:.tmp.$(notdir $@)
	/bin/echo -ne "\x10\x00\x10\x00" | cat - .tmp.$(notdir $@) > $@
