### NESICIDE auto-generated makefile 
### 
### hello_world
### 
### To make changes to the content of this file
### either change the Project Properties in
### hello_world or modify the custom rules
### file associated with the project.

# Toolchain stuff.
COMPILE  := cl65
ASSEMBLE := ca65 
LINK     := cl65

# Environment stuff.
RM = rm

ifeq ($(shell echo),)
  MKDIR = mkdir -p $1
  RMDIR = -rmdir $1
  RMFILES = -$(RM) $1
else
  MKDIR = mkdir $(subst /,\,$1)
  RMDIR = -rmdir $(subst /,\,$1)
  RMFILES = -$(if $1,del /f $(subst /,\,$1))
endif

# Path(s) to additional libraries required for linking the program
# Use only if you don't want to place copies of the libraries in SRCDIR
# Default: none
LIBS    :=

# Get rid of Windows path wonkiness.
LIBS := $(subst \,/,$(LIBS))

# Custom linker configuration file
# Use only if you don't want to place it in SRCDIR
# Default: none
CONFIG  := nes.ini

# Get rid of Windows path wonkiness.
CONFIG := $(subst \,/,$(CONFIG))

# Additional C compiler flags and options.
# Default: none
CFLAGS  = -t nes  -g --debug-info  -I .

# Get rid of Windows path wonkiness.
CFLAGS := $(subst \,/,$(CFLAGS))

# Additional assembler flags and options.
# Default: none
ASFLAGS = -t nes  -g --debug-info  -I .

# Get rid of Windows path wonkiness.
ASFLAGS := $(subst \,/,$(ASFLAGS))

# Additional linker flags and options.
# Default: none
LDFLAGS = -t nes -C $(CONFIG)  -Wl --dbgfile,hello_world.dbg
REMOVES += hello_world.dbg

# Get rid of Windows path wonkiness.
LDFLAGS := $(subst \,/,$(LDFLAGS))

# Path to the directory where object files are to be stored.
OBJDIR := obj/nes

# Get rid of Windows path wonkiness.
OBJDIR := $(subst \,/,$(OBJDIR))

# Path to the directory where PRG files are to be stored.
PRGDIR := obj/nes

# Get rid of Windows path wonkiness.
PRGDIR := $(subst \,/,$(PRGDIR))

# Program ROM file name (game code goes here).
PROGRAM = $(PRGDIR)/hello_world.prg

# Set SOURCES to something like 'src/foo.c src/bar.s'.
SOURCES := 
SOURCES += src/main.s header.s
SOURCES += 

# Get rid of Windows path wonkiness.
SOURCES := $(subst \,/,$(SOURCES))

# Set OBJECTS to something like 'obj/foo.o obj/bar.o'.
OBJECTS := $(addsuffix .o,$(basename $(addprefix $(OBJDIR)/,$(notdir $(SOURCES)))))

# Set DEPENDS to something like 'obj/foo.d obj/bar.d'.
DEPENDS := $(OBJECTS:.o=.d)

### START USER-SUPPLIED CUSTOM RULES SECTION



### END USER-SUPPLIED CUSTOM RULES SECTION

.SUFFIXES:
.PHONY: all clean
	
all: $(OBJDIR) $(PROGRAM)

# CPTODO: Disabled for now because of Windows crap paths
#-include $(DEPENDS)

# The remaining targets.
$(OBJDIR):
	$(call MKDIR,$(OBJDIR))

vpath %c $(foreach c,$(SOURCES),$(dir $c))

$(OBJDIR)/%.o: %.c
	$(COMPILE) --create-dep $(@:.o=.d) -S $(CFLAGS) -o $(@:.o=.s) $<
	$(ASSEMBLE) $(ASFLAGS) -o $@ $(@:.o=.s)

vpath %c65 $(foreach c65,$(SOURCES),$(dir $c65))

$(OBJDIR)/%.o: %.c65
	$(COMPILE) --create-dep $(@:.o=.d) -S $(CFLAGS) -o $(@:.o=.s) $<
	$(ASSEMBLE) $(ASFLAGS) -o $@ $(@:.o=.s)

vpath %a $(foreach a,$(SOURCES),$(dir $a))

$(OBJDIR)/%.o: %.a
	$(ASSEMBLE) $(ASFLAGS) -o $@ $<

vpath %asm $(foreach asm,$(SOURCES),$(dir $asm))

$(OBJDIR)/%.o: %.asm
	$(ASSEMBLE) $(ASFLAGS) -o $@ $<

vpath %a65 $(foreach a65,$(SOURCES),$(dir $a65))

$(OBJDIR)/%.o: %.a65
	$(ASSEMBLE) $(ASFLAGS) -o $@ $<

vpath %s $(foreach s,$(SOURCES),$(dir $s))

$(OBJDIR)/%.o: %.s
	$(ASSEMBLE) $(ASFLAGS) -o $@ $<

vpath %s65 $(foreach s65,$(SOURCES),$(dir $s65))

$(OBJDIR)/%.o: %.s65
	$(ASSEMBLE) $(ASFLAGS) -o $@ $<



$(PROGRAM): $(CONFIG) $(OBJECTS) $(LIBS)
	$(LINK) $(LDFLAGS) $(OBJECTS) $(LIBS) -o $@ 

clean:
	-$(call RMFILES,$(OBJECTS))
	-$(call RMFILES,$(DEPENDS))
	-$(call RMFILES,$(REMOVES))
	-$(call RMFILES,$(PROGRAM))
