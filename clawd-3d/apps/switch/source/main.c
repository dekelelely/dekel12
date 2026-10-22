// Clawd for Nintendo Switch (homebrew). The Rust engine (clawd-3d/rust, through clawd.h) plays his moves and draws
// him at 640×360; this program shows that picture twice the size on the 1280×720 screen and lets you play with him:
// buttons, sticks and the touch screen. Like floating Clawd on the desktop, he plays a move every few seconds on his
// own, and every tap on him plays a new one.
#include <switch.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "clawd.h"
#include "font8x8.h"

#define VIEW_W 640
#define VIEW_H 360
#define SCALE 2
#define SCREEN_W (VIEW_W * SCALE)
#define SCREEN_H (VIEW_H * SCALE)
#define FRAME_S (1.0 / 30)   // he is drawn at most 30 times a second; his moves step every 60 ms anyway
#define DRAG_START 12        // pixels a finger moves before a touch turns the stage instead of tapping

static u32 rgb(u8 r, u8 g, u8 b) {
    return (u32)r | (u32)g << 8 | (u32)b << 16 | 0xffu << 24;   // RGBA8888, as the framebuffer and the engine lay it out
}

// Clawd's picture into the framebuffer, each pixel a 2×2 block. The engine's RGBA bytes are the framebuffer's.
static void blit(u32 *fb, u32 stride, const uint8_t *picture) {
    for (int y = 0; y < VIEW_H; y++) {
        const u32 *src = (const u32 *)(picture + (size_t)y * VIEW_W * 4);
        u32 *top = (u32 *)((u8 *)fb + (size_t)y * SCALE * stride);
        for (int x = 0; x < VIEW_W; x++) top[2 * x] = top[2 * x + 1] = src[x];
        memcpy((u8 *)top + stride, top, SCREEN_W * 4);
    }
}

// Text in the 8×8 font, each font pixel 2×2.
static int text(u32 *fb, u32 stride, int x, int y, const char *s, u32 color) {
    for (; *s; s++, x += 8 * SCALE) {
        unsigned char c = (unsigned char)*s;
        if (c < 32 || c > 126) continue;
        const unsigned char *glyph = FONT8X8[c - 32];
        for (int row = 0; row < 8; row++) {
            for (int col = 0; col < 8; col++) {
                if (!(glyph[row] & (1 << col))) continue;
                for (int dy = 0; dy < SCALE; dy++) {
                    u32 *line = (u32 *)((u8 *)fb + (size_t)(y + row * SCALE + dy) * stride);
                    for (int dx = 0; dx < SCALE; dx++) line[x + col * SCALE + dx] = color;
                }
            }
        }
    }
    return x;
}

// The move playing now and the switches that are on (top left, like the web page's prompt), and the controls.
static void hints(u32 *fb, u32 stride, ClawdView *view) {
    const u32 clay = rgb(0xd7, 0x77, 0x57), muted = rgb(0x9b, 0xa2, 0xac);
    char line[64];
    snprintf(line, sizeof line, "> %s", clawd_current(view));
    int x = text(fb, stride, 24, 20, line, muted);
    if (clawd_auto(view)) x = text(fb, stride, x + 32, 20, "auto-play", clay);
    if (clawd_ultra(view)) x = text(fb, stride, x + 32, 20, "ultra", clay);
    if (clawd_working(view)) text(fb, stride, x + 32, 20, "working", clay);
    text(fb, stride, 24, SCREEN_H - 36, "A move  L/R pick  X auto  Y ultra  B laptop  - hints  + quit", muted);
}

static void play_index(ClawdView *view, int index) {
    const char *name = clawd_move_name(index);
    if (name) {
        clawd_set_auto(view, 0);
        clawd_play(view, name);
    }
}

int main(int argc, char **argv) {
    (void)argc;
    (void)argv;
    PadState pad;
    padConfigureInput(1, HidNpadStyleSet_NpadStandard);
    padInitializeDefault(&pad);
    hidInitializeTouchScreen();

    Framebuffer fb;
    framebufferCreate(&fb, nwindowGetDefault(), SCREEN_W, SCREEN_H, PIXEL_FORMAT_RGBA_8888, 2);
    framebufferMakeLinear(&fb);

    u64 seed = armGetSystemTick();
    srand((unsigned)seed);
    ClawdView *view = clawd_new(CLAWD_STAGE, VIEW_W, VIEW_H, VIEW_W, VIEW_H, CLAWD_BACKGROUND_APP, 1024, 1, (uint32_t)seed);
    clawd_set_ambient(view, 1);
    clawd_entrance(view);

    const int moves = clawd_move_count();
    int picked = 0;                 // the move L and R step through
    int show_hints = 1;
    const uint8_t *picture = clawd_render(view);
    int fresh = 2;                  // framebuffers still showing an older picture (there are two)
    int behind = 0;                 // he changed since he was last drawn
    double since_drawn = 0;
    char shown[96] = "";            // the hint text on screen, to notice when it changes
    struct { int down, moved; float x0, y0, x, y; } touch = {0};
    u64 last = armGetSystemTick();

    while (appletMainLoop()) {
        padUpdate(&pad);
        u64 down = padGetButtonsDown(&pad);
        if (down & HidNpadButton_Plus) break;
        if (down & HidNpadButton_A) play_index(view, rand() % moves);
        if (down & HidNpadButton_L) play_index(view, picked = (picked + moves - 1) % moves);
        if (down & HidNpadButton_R) play_index(view, picked = (picked + 1) % moves);
        if (down & HidNpadButton_X) clawd_set_auto(view, !clawd_auto(view));
        if (down & HidNpadButton_Y) clawd_set_ultra(view, !clawd_ultra(view));
        if (down & HidNpadButton_B) clawd_work(view, !clawd_working(view));
        if (down & HidNpadButton_Minus) {
            show_hints = !show_hints;
            fresh = 2;
        }

        u64 now = armGetSystemTick();
        double dt = armTicksToNs(now - last) / 1e9;
        last = now;

        // The right stick turns the stage; the left one is where he looks.
        HidAnalogStickState right = padGetStickPos(&pad, 1), left = padGetStickPos(&pad, 0);
        if (abs(right.x) > 4000 || abs(right.y) > 4000) {
            clawd_orbit(view, right.x / (double)JOYSTICK_MAX * 300 * dt, right.y / (double)JOYSTICK_MAX * 200 * dt, 1);
        }
        int looking = abs(left.x) > 6000 || abs(left.y) > 6000;
        if (looking) clawd_gaze(view, VIEW_W / 2 + left.x / (double)JOYSTICK_MAX * 400, VIEW_H / 2 - left.y / (double)JOYSTICK_MAX * 400);

        // Touch: a tap on him plays a move; a drag turns the stage; his eyes follow your finger meanwhile.
        HidTouchScreenState ts = {0};
        if (hidGetTouchScreenStates(&ts, 1) && ts.count > 0) {
            float x = ts.touches[0].x, y = ts.touches[0].y;
            if (!touch.down) {
                touch.down = 1;
                touch.moved = 0;
                touch.x0 = x;
                touch.y0 = y;
            } else {
                float mx = x - touch.x0, my = y - touch.y0;
                if (!touch.moved && mx * mx + my * my > DRAG_START * DRAG_START) touch.moved = 1;
                if (touch.moved) clawd_orbit(view, (x - touch.x) / SCALE, (y - touch.y) / SCALE, 1);
            }
            touch.x = x;
            touch.y = y;
            clawd_gaze(view, x / SCALE, y / SCALE);
        } else if (touch.down) {
            touch.down = 0;
            if (!touch.moved) clawd_tap(view, touch.x / SCALE, touch.y / SCALE);
            clawd_look_ahead(view);
        } else if (!looking) {
            clawd_look_ahead(view);
        }

        if (clawd_update(view, dt)) behind = 1;
        since_drawn += dt;
        if (behind && since_drawn >= FRAME_S - 0.002) {
            picture = clawd_render(view);
            behind = 0;
            since_drawn = 0;
            fresh = 2;
        }
        char state[96];
        snprintf(state, sizeof state, "%s %d %d %d %d", clawd_current(view), clawd_auto(view), clawd_ultra(view),
                 clawd_working(view), show_hints);
        if (strcmp(state, shown)) {
            strcpy(shown, state);
            fresh = 2;
        }

        u32 stride;
        u32 *pixels = (u32 *)framebufferBegin(&fb, &stride);
        if (fresh > 0) {
            blit(pixels, stride, picture);
            if (show_hints) hints(pixels, stride, view);
            fresh--;
        }
        framebufferEnd(&fb);
    }

    clawd_free(view);
    framebufferClose(&fb);
    return 0;
}
