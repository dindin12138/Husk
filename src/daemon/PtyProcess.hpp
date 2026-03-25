#pragma once

#include <deque>
#include <mutex>
#include <string>
#include <sys/types.h>
#include <variant>

namespace husk::daemon {

struct CmdInput {
  std::string data;
};
struct CmdResize {
  uint16_t cols;
  uint16_t rows;
};
using DaemonCommand = std::variant<CmdInput, CmdResize>;

class PtyProcess {
public:
  PtyProcess(uint16_t cols, uint16_t rows);
  ~PtyProcess();

  PtyProcess(const PtyProcess &) = delete;
  PtyProcess &operator=(const PtyProcess &) = delete;

  int get_pty_fd() const { return m_pty_fd; }
  int get_wakeup_fd() const { return m_wakeup_fd; }
  pid_t get_pid() const { return m_pid; }

  void enqueue_command(DaemonCommand cmd);

  std::deque<DaemonCommand> acquire_all_commands();

private:
  int m_pty_fd{-1};
  int m_wakeup_fd{-1};
  pid_t m_pid{-1};

  std::deque<DaemonCommand> m_cmd_queue;
  std::mutex m_cmd_mutex;

  bool m_is_sleeping{true};
};

} // namespace husk::daemon
