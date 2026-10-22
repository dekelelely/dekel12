/* Clawd's Rust engine and renderer, from C. One ClawdView is one Clawd: he plays moves, and draws himself into an
 * RGBA picture of the size you give. Coordinates (taps, gaze) are in view units: width × height as passed to
 * clawd_new; the picture is pixel_width × pixel_height. Not thread-safe: call from one thread. */
#ifndef CLAWD_H
#define CLAWD_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ClawdView ClawdView;

enum { CLAWD_STAGE = 0, CLAWD_PET = 1 };
enum { CLAWD_BACKGROUND_TRANSPARENT = 0, CLAWD_BACKGROUND_APP = 1 };

/* mode: CLAWD_STAGE (the floor strip) or CLAWD_PET (just him and his shadow). shadow_size: 0 for the default. */
ClawdView *clawd_new(int mode, double width, double height, int pixel_width, int pixel_height, int background,
                     int shadow_size, int soft_shadows, uint32_t seed);
void clawd_free(ClawdView *view);
void clawd_resize(ClawdView *view, double width, double height, int pixel_width, int pixel_height);

/* Moves time on (seconds). Returns 1 if the picture changed since the last frame. */
int clawd_update(ClawdView *view, double dt);
/* Draws him: pixel_width × pixel_height RGBA (not premultiplied), valid until the next call. */
const uint8_t *clawd_render(ClawdView *view);

/* Plays a move by name ("idle" too). Returns 0 if there is no such move. */
int clawd_play(ClawdView *view, const char *name);
/* A random move to start with. */
void clawd_entrance(ClawdView *view);
/* The move playing now, e.g. "jump" or "idle" (a static string). */
const char *clawd_current(const ClawdView *view);
int clawd_move_count(void);
const char *clawd_move_name(int index);

/* A tap at (x, y): if it lands on him he plays a move. Returns 1 if it hit him. */
int clawd_tap(ClawdView *view, double x, double y);
/* Turns the stage: dx, dy in view units; tilt also changes the height of the view. */
void clawd_orbit(ClawdView *view, double dx, double dy, int tilt);
/* His eyes follow (x, y); clawd_look_ahead to stop. */
void clawd_gaze(ClawdView *view, double x, double y);
void clawd_look_ahead(ClawdView *view);

void clawd_set_auto(ClawdView *view, int on);
int clawd_auto(const ClawdView *view);
/* Ultra effort: the guitar routine, and the guitar kept between moves. */
void clawd_set_ultra(ClawdView *view, int on);
int clawd_ultra(const ClawdView *view);
/* Floating: he plays a random move every so often on his own. */
void clawd_set_ambient(ClawdView *view, int on);
/* Working: he types on his laptop until told to stop. */
void clawd_work(ClawdView *view, int on);
int clawd_working(const ClawdView *view);

#ifdef __cplusplus
}
#endif

#endif
