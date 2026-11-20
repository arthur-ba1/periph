// SPDX-License-Identifier: GPL-2.0-or-later
// Akko's default keymap uses RGB Matrix names introduced after its QMK base.
#define RM_NEXT RGB_MOD
#define RM_SPDD RGB_SPD
#define RM_SPDU RGB_SPI
#define RM_TOGG RGB_TOG
#define RM_HUEU RGB_HUI
#define RM_VALU RGB_VAI
#define RM_VALD RGB_VAD
#define RM_SATU RGB_SAI
#define RM_SATD RGB_SAD
#include "../default/keymap.c"

#include <string.h>

#define WORD_CAPACITY 32
#define WORD_LIGHT_MS 380u
#define WORD_GAP_MS 100u
#define WORD_REPEAT_GAP_MS 800u

static keypos_t word[WORD_CAPACITY] = { // ARTHUR, in physical matrix positions.
    {.row = 3, .col = 1}, {.row = 2, .col = 4}, {.row = 2, .col = 5},
    {.row = 3, .col = 6}, {.row = 2, .col = 7}, {.row = 2, .col = 4},
};
static uint8_t word_length = 6;
static keypos_t draft[WORD_CAPACITY];
static uint8_t draft_length;
static bool editing;
static bool restart_word;
static uint16_t consumed[MATRIX_ROWS];
static uint32_t cycle_start;

uint8_t word_led_at(uint32_t now, bool init) {
    if (restart_word || init) {
        cycle_start = now;
        restart_word = false;
    }

    const uint32_t step_ms = WORD_LIGHT_MS + WORD_GAP_MS;
    const uint32_t cycle_ms = word_length * step_ms - WORD_GAP_MS + WORD_REPEAT_GAP_MS;
    const uint32_t elapsed = (now - cycle_start) % cycle_ms;
    cycle_start = now - elapsed;
    const uint32_t step = elapsed / step_ms;
    if (step >= word_length || elapsed % step_ms >= WORD_LIGHT_MS) return NO_LED;

    keypos_t position = word[step];
    return g_led_config.matrix_co[position.row][position.col];
}

static bool is_word_key(uint16_t keycode) {
    return (keycode >= KC_A && keycode <= KC_Z) || (keycode >= KC_1 && keycode <= KC_0);
}

bool process_record_user(uint16_t keycode, keyrecord_t *record) {
    const keypos_t position = record->event.key;
    if (position.row >= MATRIX_ROWS || position.col >= MATRIX_COLS) return true;
    const uint16_t bit = (uint16_t)1u << position.col;
    uint16_t *row = &consumed[position.row];

    if (!record->event.pressed) {
        const bool was_consumed = (*row & bit) != 0;
        *row &= ~bit;
        return !was_consumed;
    }

    if (keycode == KC_BSPC && (layer_state_is(WIN_FN) || layer_state_is(MAC_FN))) {
        editing = true;
        draft_length = 0;
    } else if (!editing) {
        return true;
    } else if (keycode == KC_ESC || (position.row == 0 && position.col == 0)) {
        editing = false;
    } else if (keycode == KC_ENT) {
        editing = false;
        if (draft_length) {
            memcpy(word, draft, draft_length * sizeof(*word));
            word_length = draft_length;
            restart_word = true;
            rgb_matrix_enable_noeeprom();
            rgb_matrix_mode_noeeprom(RGB_MATRIX_CUSTOM_WORD);
        }
    } else if (keycode == KC_BSPC) {
        if (draft_length) --draft_length;
    } else if (draft_length < WORD_CAPACITY && is_word_key(keycode) &&
               g_led_config.matrix_co[position.row][position.col] != NO_LED) {
        draft[draft_length++] = position;
    }

    *row |= bit;
    return false;
}
