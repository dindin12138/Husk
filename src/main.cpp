#include "terminal/Terminal.hpp"
#include <iostream>

int main() {
  std::cout << "Starting Husk...\n";

  try {
    husk::Terminal term(80, 24);

    term.write("\x1b[31;1mHello from C++ Terminal!\x1b[0m\r\n");
    term.resize(100, 30);

    std::cout
        << "Terminal initialized, resized, and data written successfully.\n";

  } catch (const std::exception &e) {
    std::cerr << "Fatal Error: " << e.what() << '\n';
    return 1;
  }

  return 0;
}
