// SPDX-License-Identifier: GPL-2.0-or-later
RGB_MATRIX_EFFECT(WORD)

#ifdef RGB_MATRIX_CUSTOM_EFFECT_IMPLS
uint8_t word_led_at(uint32_t now, bool init);

static bool WORD(effect_params_t *params) {
    static uint8_t active_led;
    if (params->iter == 0) active_led = word_led_at(g_rgb_timer, params->init);

    const RGB color = rgb_matrix_hsv_to_rgb(rgb_matrix_config.hsv);
    RGB_MATRIX_USE_LIMITS(led_min, led_max);
    for (uint8_t i = led_min; i < led_max; ++i) {
        RGB_MATRIX_TEST_LED_FLAGS();
        if (i == active_led) {
            rgb_matrix_set_color(i, color.r, color.g, color.b);
        } else {
            rgb_matrix_set_color(i, 0, 0, 0);
        }
    }
    return rgb_matrix_check_finished_leds(led_max);
}
#endif
