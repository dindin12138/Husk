#pragma once

#include <cstdint>
#include <stdexcept>

extern "C" {
#include <ghostty/vt/render.h>
}

#include "Terminal.hpp"

namespace husk::vt {

class RowIterator;
class RowCells;

class RenderState {
public:
  RenderState();
  ~RenderState();

  RenderState(const RenderState &) = delete;
  RenderState &operator=(const RenderState &) = delete;

  void update(const husk::vt::Terminal &terminal);

  uint16_t cursor_x() const;
  uint16_t cursor_y() const;
  bool is_cursor_visible() const;

  void populate_iterator(RowIterator &iter) const;

private:
  GhosttyRenderState m_state{nullptr};
};

class RowIterator {
public:
  RowIterator();
  ~RowIterator();

  RowIterator(const RowIterator &) = delete;
  RowIterator &operator=(const RowIterator &) = delete;

  bool next();

  void populate_cells(RowCells &cells) const;

  GhosttyRenderStateRowIterator *handle_ptr() { return &m_iter; }

private:
  GhosttyRenderStateRowIterator m_iter{nullptr};
};

class RowCells {
public:
  RowCells();
  ~RowCells();

  RowCells(const RowCells &) = delete;
  RowCells &operator=(const RowCells &) = delete;

  bool next();

  uint32_t bg_color() const;
  uint32_t fg_color() const;
  uint32_t codepoint() const;

  GhosttyRenderStateRowCells *handle_ptr() { return &m_cells; }

private:
  GhosttyRenderStateRowCells m_cells{nullptr};
};

} // namespace husk::vt
