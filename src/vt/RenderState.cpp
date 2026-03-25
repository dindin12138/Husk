#include "RenderState.hpp"
#include <vector>

namespace husk::vt {

// ================= RenderState =================

RenderState::RenderState() {
  if (ghostty_render_state_new(nullptr, &m_state) != GHOSTTY_SUCCESS) {
    throw std::runtime_error("Failed to allocate GhosttyRenderState");
  }
}

RenderState::~RenderState() {
  if (m_state)
    ghostty_render_state_free(m_state);
}

void RenderState::update(const husk::vt::Terminal &terminal) {
  ghostty_render_state_update(m_state, terminal.get_handle());
}

uint16_t RenderState::cursor_x() const {
  uint16_t x = 0;
  bool has_value = false;
  ghostty_render_state_get(
      m_state, GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE, &has_value);
  if (has_value) {
    ghostty_render_state_get(m_state,
                             GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X, &x);
  }
  return x;
}

uint16_t RenderState::cursor_y() const {
  uint16_t y = 0;
  bool has_value = false;
  ghostty_render_state_get(
      m_state, GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE, &has_value);
  if (has_value) {
    ghostty_render_state_get(m_state,
                             GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y, &y);
  }
  return y;
}

bool RenderState::is_cursor_visible() const {
  bool visible = false;
  ghostty_render_state_get(m_state, GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE,
                           &visible);
  return visible;
}

void RenderState::populate_iterator(RowIterator &iter) const {
  ghostty_render_state_get(m_state, GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                           iter.handle_ptr());
}

// ================= RowIterator =================

RowIterator::RowIterator() {
  if (ghostty_render_state_row_iterator_new(nullptr, &m_iter) !=
      GHOSTTY_SUCCESS) {
    throw std::runtime_error(
        "Failed to allocate GhosttyRenderStateRowIterator");
  }
}

RowIterator::~RowIterator() {
  if (m_iter)
    ghostty_render_state_row_iterator_free(m_iter);
}

bool RowIterator::next() {
  return ghostty_render_state_row_iterator_next(m_iter);
}

void RowIterator::populate_cells(RowCells &cells) const {
  ghostty_render_state_row_get(m_iter, GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                               cells.handle_ptr());
}

// ================= RowCells =================

RowCells::RowCells() {
  if (ghostty_render_state_row_cells_new(nullptr, &m_cells) !=
      GHOSTTY_SUCCESS) {
    throw std::runtime_error("Failed to allocate GhosttyRenderStateRowCells");
  }
}

RowCells::~RowCells() {
  if (m_cells)
    ghostty_render_state_row_cells_free(m_cells);
}

bool RowCells::next() { return ghostty_render_state_row_cells_next(m_cells); }

uint32_t RowCells::bg_color() const {
  GhosttyColorRgb color;
  if (ghostty_render_state_row_cells_get(
          m_cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR, &color) ==
      GHOSTTY_SUCCESS) {
    return (color.r << 24) | (color.g << 16) | (color.b << 8) | 0xFF;
  }
  return 0x000000FF;
}

uint32_t RowCells::fg_color() const {
  GhosttyColorRgb color;
  if (ghostty_render_state_row_cells_get(
          m_cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR, &color) ==
      GHOSTTY_SUCCESS) {
    return (color.r << 24) | (color.g << 16) | (color.b << 8) | 0xFF;
  }
  return 0xFFFFFFFF;
}

uint32_t RowCells::codepoint() const {
  uint32_t graphemes_len = 0;
  if (ghostty_render_state_row_cells_get(
          m_cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
          &graphemes_len) == GHOSTTY_SUCCESS &&
      graphemes_len > 0) {
    std::vector<uint32_t> buf(graphemes_len);
    if (ghostty_render_state_row_cells_get(
            m_cells, GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
            buf.data()) == GHOSTTY_SUCCESS) {
      return buf[0];
    }
  }
  return 0;
}
} // namespace husk::vt
