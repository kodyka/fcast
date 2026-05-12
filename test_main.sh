xvfb-run --auto-servernum slint-viewer senders/android/ui/main.slint > viewer_main.log 2>&1 &
sleep 5
cat viewer_main.log
