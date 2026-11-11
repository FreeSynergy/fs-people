# Scenario of the typing test (fs-release actions/typing-test): defines focus(), which brings a text field of the window
# into focus. The people window opens on the list with the field "Search people" at the top left. Positions are relative
# to the window's top left corner (WX0, WY0, set by the test; title bar included), for the window size of the workflow
# (`--size 1000x640`). The field is 44 px high (y 52..96 of the picture); the point lies inside with or without a title bar.
focus() {
  local x=$((WX0 + 170)) y=$((WY0 + 88))
  TT_NEAR="$x,$y"; export TT_NEAR
  c "move $x $y" 0.4
  c "click $x $y" 1
}
