#!/bin/bash
slint-viewer senders/android/ui/pages/audio_page.slint > viewer_audio.log 2>&1 &
slint-viewer senders/android/ui/pages/recording_page.slint > viewer_rec.log 2>&1 &
slint-viewer senders/android/ui/pages/cast_history_page.slint > viewer_cast.log 2>&1 &
