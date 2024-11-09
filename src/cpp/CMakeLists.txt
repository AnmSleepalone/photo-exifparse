# CMakeList.txt: ImgHandel 的 CMake 项目，在此处包括源代码并定义
# 项目特定的逻辑。
#
cmake_minimum_required (VERSION 3.8)
# 如果支持，请为 MSVC 编译器启用热重载。
if (POLICY CMP0141)
  cmake_policy(SET CMP0141 NEW)
  set(CMAKE_MSVC_DEBUG_INFORMATION_FORMAT "$<IF:$<AND:$<C_COMPILER_ID:MSVC>,$<CXX_COMPILER_ID:MSVC>>,$<$<CONFIG:Debug,RelWithDebInfo>:EditAndContinue>,$<$<CONFIG:Debug,RelWithDebInfo>:ProgramDatabase>>")
endif()

project ("ImgHandel")
#set(CMAKE_TOOLCHAIN_FILE "C:/Users/Administrator/vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "Vcpkg toolchain file")
set(CMAKE_PREFIX_PATH "D:/code/interesting/ImgHandel/vcpkg_installed/vcpkg/pkgs/libjpeg-turbo_x64-windows/")


# 将源代码添加到此项目的可执行文件。
add_executable (ImgHandel "ImgHandel.cpp" "ImgHandel.h")

find_package(libjpeg-turbo CONFIG REQUIRED)
target_link_libraries(ImgHandel PRIVATE $<IF:$<TARGET_EXISTS:libjpeg-turbo::turbojpeg>,libjpeg-turbo::turbojpeg,libjpeg-turbo::turbojpeg-static>)








# 添加编译选项，启用 SIMD 支持
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(ImgHandel PRIVATE 
        -march=native
        -O3
    )
endif()


if (CMAKE_VERSION VERSION_GREATER 3.12)
  set_property(TARGET ImgHandel PROPERTY CXX_STANDARD 20)
endif()

# TODO: 如有需要，请添加测试并安装目标。
