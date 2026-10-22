#---------------------------------------------------------------------------------
# Clawd for Nintendo Switch: devkitPro's libnx application template, plus the Rust engine (clawd-3d/rust/clawd-ffi)
# built first as a static library and linked in. Needs devkitA64 + libnx, and rustup (see README.md).
#   make                     clawd.nro
#   make APP_VERSION=1.23.0  with that version in the Homebrew Menu
#---------------------------------------------------------------------------------
.SUFFIXES:

ifeq ($(strip $(DEVKITPRO)),)
$(error "Please set DEVKITPRO in your environment. export DEVKITPRO=<path to>/devkitpro")
endif

TOPDIR ?= $(CURDIR)
include $(DEVKITPRO)/libnx/switch_rules

TARGET		:=	clawd
BUILD		:=	build
SOURCES		:=	source
DATA		:=	data
INCLUDES	:=	../../rust/clawd-ffi
APP_TITLE	:=	Clawd
APP_AUTHOR	:=	dekelelely
APP_VERSION	?=	1.0.0

#---------------------------------------------------------------------------------
# The Rust engine. core and alloc are rebuilt with it (nightly's -Z build-std) so that every object is
# position-independent: the prebuilt ones for aarch64-unknown-none aren't, and an NRO is a PIE whose read-only data
# can't be patched when it loads.
#---------------------------------------------------------------------------------
RUST_TOOLCHAIN	?=	nightly-2026-10-15
RUST_DIR	:=	$(TOPDIR)/../../rust
RUST_OUT	:=	$(RUST_DIR)/target/switch/aarch64-unknown-none/release
RUST_LIB	:=	$(RUST_OUT)/libclawd_ffi.a

ARCH	:=	-march=armv8-a+crc+crypto -mtune=cortex-a57 -mtp=soft -fPIE

CFLAGS	:=	-g -Wall -O2 -ffunction-sections \
			$(ARCH) $(DEFINES)

CFLAGS	+=	$(INCLUDE) -D__SWITCH__

CXXFLAGS	:= $(CFLAGS) -fno-rtti -fno-exceptions

ASFLAGS	:=	-g $(ARCH)
LDFLAGS	=	-specs=$(DEVKITPRO)/libnx/switch.specs -g $(ARCH) -Wl,-Map,$(notdir $*.map) -L$(RUST_OUT)

# The engine first: it needs only malloc's aligned_alloc, free and abort from the C library.
LIBS	:= -lclawd_ffi -lnx

LIBDIRS	:= $(PORTLIBS) $(LIBNX)

ifneq ($(BUILD),$(notdir $(CURDIR)))

export OUTPUT	:=	$(CURDIR)/$(TARGET)
export TOPDIR	:=	$(CURDIR)

export VPATH	:=	$(foreach dir,$(SOURCES),$(CURDIR)/$(dir)) \
			$(foreach dir,$(DATA),$(CURDIR)/$(dir))

export DEPSDIR	:=	$(CURDIR)/$(BUILD)

CFILES		:=	$(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.c)))
CPPFILES	:=	$(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.cpp)))
SFILES		:=	$(foreach dir,$(SOURCES),$(notdir $(wildcard $(dir)/*.s)))
BINFILES	:=	$(foreach dir,$(DATA),$(notdir $(wildcard $(dir)/*.*)))

ifeq ($(strip $(CPPFILES)),)
	export LD	:=	$(CC)
else
	export LD	:=	$(CXX)
endif

export OFILES_BIN	:=	$(addsuffix .o,$(BINFILES))
export OFILES_SRC	:=	$(CPPFILES:.cpp=.o) $(CFILES:.c=.o) $(SFILES:.s=.o)
export OFILES 	:=	$(OFILES_BIN) $(OFILES_SRC)
export HFILES_BIN	:=	$(addsuffix .h,$(subst .,_,$(BINFILES)))

export INCLUDE	:=	$(foreach dir,$(INCLUDES),-I$(CURDIR)/$(dir)) \
			$(foreach dir,$(LIBDIRS),-I$(dir)/include) \
			-I$(CURDIR)/$(BUILD)

export LIBPATHS	:=	$(foreach dir,$(LIBDIRS),-L$(dir)/lib)

export APP_ICON := $(TOPDIR)/icon.jpg
export NROFLAGS += --icon=$(APP_ICON)
export NROFLAGS += --nacp=$(CURDIR)/$(TARGET).nacp

.PHONY: $(BUILD) rust clean all

all: $(BUILD)

# cargo decides itself whether anything needs rebuilding.
rust:
	@echo building the Rust engine ...
	@cd $(RUST_DIR) && RUSTFLAGS="-C relocation-model=pic -C target-cpu=cortex-a57" \
		cargo +$(RUST_TOOLCHAIN) build --release -p clawd-ffi --target aarch64-unknown-none \
		-Z build-std=core,alloc --target-dir target/switch

$(BUILD): rust
	@[ -d $@ ] || mkdir -p $@
	@$(MAKE) --no-print-directory -C $(BUILD) -f $(CURDIR)/Makefile

clean:
	@echo clean ...
	@rm -fr $(BUILD) $(TARGET).nro $(TARGET).nacp $(TARGET).elf
	@rm -fr $(RUST_DIR)/target/switch

#---------------------------------------------------------------------------------
else
.PHONY:	all

DEPENDS	:=	$(OFILES:.o=.d)

all	:	$(OUTPUT).nro

$(OUTPUT).nro	:	$(OUTPUT).elf $(OUTPUT).nacp

# Relinks when the engine changes.
$(OUTPUT).elf	:	$(OFILES) $(RUST_LIB)

$(OFILES_SRC)	: $(HFILES_BIN)

%.bin.o	%_bin.h :	%.bin
	@echo $(notdir $<)
	@$(bin2o)

-include $(DEPENDS)

#---------------------------------------------------------------------------------
endif
#---------------------------------------------------------------------------------
