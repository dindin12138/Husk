#include "common/SharedGrid.hpp"
#include "daemon/PtyProcess.hpp"
#include "daemon/TerminalMachine.hpp"
#include <errno.h>
#include <iostream>
#include <string.h>
#include <unistd.h>
#include <vector>

using namespace husk::daemon;
using namespace husk::common;

int main() {
  std::cout << "[Husk] Booting Terminal Engine (Shell '-c' Mode)..."
            << std::endl;

  uint16_t cols = 80;
  uint16_t rows = 24;

  PtyProcess pty(cols, rows);
  TerminalMachine vt(cols, rows);
  SharedState shm(cols, rows);

  std::cout << "[Husk] Waiting 0.5s for Shell to execute and output..."
            << std::endl;
  usleep(500000);

  char dump_buf[4096];
  ssize_t bytes_read;
  size_t total_bytes = 0;

  std::cout << "[Husk] Draining output..." << std::endl;

  for (int i = 0; i < 50; ++i) {
    bytes_read = read(pty.get_pty_fd(), dump_buf, sizeof(dump_buf));

    if (bytes_read > 0) {
      std::cout << "  -> Drained " << bytes_read << " bytes." << std::endl;
      total_bytes += bytes_read;
      vt.feed_input(std::string_view(dump_buf, bytes_read));
    } else if (bytes_read == 0) {
      std::cout << "  -> EOF reached (Shell finished the command and exited)."
                << std::endl;
      break;
    }
    usleep(10000);
  }

  std::cout << "[Husk] Total PTY Emitted: " << total_bytes << " bytes."
            << std::endl;

  GridSnapshot *back_buffer = shm.get_back_buffer_for_write(cols, rows);
  vt.snapshot_to_buffer(back_buffer);
  shm.commit_back_buffer();

  GridSnapshot *front = shm.acquire_front_buffer();
  if (front) {
    std::cout << "\n================ [Husk Logical View] ================\n";
    for (int r = 0; r < 24; ++r) {
      for (int c = 0; c < front->cols; ++c) {
        uint32_t cp = front->cells[r * cols + c].codepoint;
        if (cp == 0 || cp < 32 || cp == 127)
          std::cout << ' ';
        else
          std::cout << static_cast<char>(cp);
      }
      std::cout << "|\n";
    }
    std::cout << "=====================================================\n";
  }

  std::cout << "[Husk] Shell Validation Complete." << std::endl;
  return 0;
}
