// src/cli.rs

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    about = "dlog - 开发者日志工具",
    long_about = r#"
dlog - 专为开发者设计的命令行日志工具

一个轻量级的本地日志系统，帮助您记录开发过程中的重要信息。
每条日志都与特定目录关联，让您能够按项目或功能模块组织笔记。

主要特性：
  • 目录关联：每条日志自动关联到当前工作目录
  • 标签系统：使用标签分类和组织日志
  • 递归查询：支持在目录树中搜索相关日志
  • 安全删除：删除操作需要确认，避免误删
  • 离线存储：所有数据存储在本地SQLite数据库

使用示例：
  dlog init                    # 初始化数据库
  dlog log -m              # 记录一条简单日志
  dlog log                    # 使用编辑器记录详细日志
  dlog get -r                 # 递归查看当前目录及子目录的日志
  dlog get -t bugfix          # 查看所有带有bugfix标签的日志
  dlog del 3,5-7             # 删除ID为3、5、6、7的日志

数据库位置：~/.config/dlog/dlog.db
    "#
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 初始化dlog数据库和目录同步
    ///
    /// 此命令将：
    /// 1. 在 ~/.config/dlog/dlog.db 创建数据库
    /// 2. 检查是否存在指向已删除目录的日志
    /// 3. 提示您清理这些孤立的日志条目
    ///
    /// 示例：
    ///   dlog init
    Init,

    /// 添加新的日志条目到当前目录
    ///
    /// 如果没有提供 -m 参数，将打开默认编辑器（$EDITOR）供您输入详细内容。
    /// 日志会自动关联到当前工作目录，方便后续按项目查找。
    ///
    /// 示例：
    ///   dlog log -m "完成了用户认证模块" -t "feature,auth"
    ///   dlog log                              # 打开编辑器输入
    ///   dlog log -t "bugfix,urgent"           # 带标签的编辑器输入
    Log {
        #[arg(short, long, 
              help = "简短的日志内容（类似git commit -m）",
              long_help = "直接提供日志内容，避免打开编辑器。适用于快速记录简短信息。")]
        message: Option<String>,

        #[arg(short, long, 
              help = "逗号分隔的标签",
              long_help = "使用标签对日志进行分类。多个标签用逗号分隔，例如：feature,backend,high-priority")]
        tags: Option<String>,
    },

    /// 检索和显示日志条目
    ///
    /// 默认显示当前目录的最新10条日志。
    /// 使用 -r 参数可以递归搜索子目录。
    /// 支持按标签、日期和关键词过滤。
    ///
    /// 示例：
    ///   dlog get                    # 当前目录的最新日志
    ///   dlog get -n 20              # 显示20条日志
    ///   dlog get -r                 # 递归搜索当前目录及子目录
    ///   dlog get -t bugfix          # 过滤包含bugfix标签的日志
    ///   dlog get --date 2024-01-15  # 显示特定日期的日志
    ///   dlog get -s "error"         # 搜索包含"error"的日志
    ///   dlog get /path/to/project   # 查看指定目录的日志
    Get {
        /// 要搜索的目录路径，默认为当前目录
        #[arg(help = "目标目录路径（相对或绝对路径）",
              long_help = "指定要搜索日志的目录。可以是相对路径（./project）或绝对路径（/home/user/project）。")]
        path: Option<String>,

        #[arg(short, long, 
              help = "显示最新的N条日志",
              long_help = "限制显示的日志数量。默认显示10条，使用0显示所有匹配的日志。")]
        num: Option<u32>,

        #[arg(short, long, 
              help = "递归搜索子目录",
              long_help = "在指定目录及其所有子目录中搜索日志。搜索结果会显示每条日志的完整路径。")]
        recursive: bool,

        #[arg(short, long, 
              help = "按标签过滤日志",
              long_help = "只显示包含指定标签的日志。支持部分匹配，例如'test'会匹配'test'、'integration-test'等。")]
        tag: Option<String>,

        #[arg(long, 
              help = "按日期过滤日志（格式：YYYY-MM-DD）",
              long_help = "只显示指定日期的日志。日期格式必须为年-月-日，例如：2024-01-15。")]
        date: Option<String>,

        #[arg(short, long, 
              help = "在内容和标签中搜索关键词",
              long_help = "在日志内容和标签中搜索包含指定关键词的条目。搜索不区分大小写。")]
        search: Option<String>,
    },

    /// 通过ID编辑现有的日志条目
    ///
    /// 使用默认编辑器打开指定的日志进行编辑。
    /// 如果内容没有变化，操作将被取消。
    ///
    /// 示例：
    ///   dlog fix 5    # 编辑ID为5的日志
    Fix {
        #[arg(help = "要编辑的日志ID",
              long_help = "要编辑的日志条目的数字ID。使用 'dlog get' 命令查看可用的ID。")]
        id: i32,
    },

    /// 删除一个或多个日志条目
    ///
    /// 支持多种删除方式：
    /// • 单个ID：dlog del 5
    /// • 逗号分隔：dlog del 3,5,8
    /// • 范围删除：dlog del 7-9（删除7、8、9）
    /// • 混合模式：dlog del 3,7-9,12
    /// • 递归删除：dlog del -r（删除当前目录及子目录所有日志）
    ///
    /// 所有删除操作都需要确认，输入 'y' 继续。
    #[command(verbatim_doc_comment)]
    Del {
        /// 要删除的日志ID列表
        #[arg(conflicts_with = "recursive", 
              value_name = "ID_LIST",
              help = "要删除的日志ID列表",
              long_help = r#"要删除的日志ID，支持多种格式：
  • 单个ID: 5
  • 逗号分隔: 3,5,8  
  • 范围: 7-9（删除7、8、9）
  • 混合: 3,7-9,12（删除3、7、8、9、12）"#)]
        ids: Option<String>,

        /// 递归删除当前目录及子目录的所有日志
        #[arg(short, long, 
              help = "递归删除当前目录及子目录的所有日志",
              long_help = "删除当前工作目录及其所有子目录中的所有日志条目。此操作不可逆，请谨慎使用。")]
        recursive: bool,
    },
}
