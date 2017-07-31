#![doc = include_str!("../readme.md")]

use super::DwarfInfo;
use std::collections::HashMap;

/// WebAssembly 自定义段类型
#[derive(Debug, Clone, PartialEq)]
pub enum CustomSectionType {
    /// .debug_info - 调试信息
    DebugInfo,
    /// .debug_line - 行号信息
    DebugLine,
    /// .debug_abbrev - 缩写表
    DebugAbbrev,
    /// .debug_str - 字符串表
    DebugStr,
    /// .debug_aranges - 地址范围
    DebugAranges,
    /// .debug_frame - 帧信息
    DebugFrame,
    /// .debug_loc - 位置列表
    DebugLoc,
    /// .debug_macro - 宏信息
    DebugMacro,
    /// .debug_pubnames - 公共名称
    DebugPubnames,
    /// .debug_pubtypes - 公共类型
    DebugPubtypes,
    /// .debug_ranges - 范围列表
    DebugRanges,
    /// external_debug_info - 外部调试信息引用
    ExternalDebugInfo,
    /// sourceMappingURL - 源映射 URL
    SourceMappingURL,
    /// 其他自定义段
    Other(String),
}

/// WebAssembly 自定义段
#[derive(Debug, Clone)]
pub struct CustomSection {
    /// 段名称
    pub name: String,
    /// 段类型
    pub section_type: CustomSectionType,
    /// 段数据
    pub data: Vec<u8>,
}

/// 外部调试信息引用
#[derive(Debug, Clone)]
pub struct ExternalDebugInfo {
    /// 外部文件 URL 或路径
    pub url: String,
}

/// 源映射信息
#[derive(Debug, Clone)]
pub struct SourceMapping {
    /// 源映射 URL
    pub url: String,
}

/// DWARF 自定义段管理器
#[derive(Debug, Clone)]
pub struct DwarfCustomSections {
    /// 所有自定义段
    pub sections: HashMap<String, CustomSection>,
    /// 外部调试信息
    pub external_debug_info: Option<ExternalDebugInfo>,
    /// 源映射信息
    pub source_mapping: Option<SourceMapping>,
}

impl CustomSectionType {
    /// 从段名称创建段类型
    pub fn from_name(name: &str) -> Self {
        match name {
            ".debug_info" => Self::DebugInfo,
            ".debug_line" => Self::DebugLine,
            ".debug_abbrev" => Self::DebugAbbrev,
            ".debug_str" => Self::DebugStr,
            ".debug_aranges" => Self::DebugAranges,
            ".debug_frame" => Self::DebugFrame,
            ".debug_loc" => Self::DebugLoc,
            ".debug_macro" => Self::DebugMacro,
            ".debug_pubnames" => Self::DebugPubnames,
            ".debug_pubtypes" => Self::DebugPubtypes,
            ".debug_ranges" => Self::DebugRanges,
            "external_debug_info" => Self::ExternalDebugInfo,
            "sourceMappingURL" => Self::SourceMappingURL,
            _ => Self::Other(name.to_string()),
        }
    }

    /// 获取段名称
    pub fn name(&self) -> &str {
        match self {
            Self::DebugInfo => ".debug_info",
            Self::DebugLine => ".debug_line",
            Self::DebugAbbrev => ".debug_abbrev",
            Self::DebugStr => ".debug_str",
            Self::DebugAranges => ".debug_aranges",
            Self::DebugFrame => ".debug_frame",
            Self::DebugLoc => ".debug_loc",
            Self::DebugMacro => ".debug_macro",
            Self::DebugPubnames => ".debug_pubnames",
            Self::DebugPubtypes => ".debug_pubtypes",
            Self::DebugRanges => ".debug_ranges",
            Self::ExternalDebugInfo => "external_debug_info",
            Self::SourceMappingURL => "sourceMappingURL",
            Self::Other(name) => name,
        }
    }

    /// 是否为 DWARF 调试段
    pub fn is_dwarf_section(&self) -> bool {
        matches!(
            self,
            Self::DebugInfo
                | Self::DebugLine
                | Self::DebugAbbrev
                | Self::DebugStr
                | Self::DebugAranges
                | Self::DebugFrame
                | Self::DebugLoc
                | Self::DebugMacro
                | Self::DebugPubnames
                | Self::DebugPubtypes
                | Self::DebugRanges
        )
    }
}

impl CustomSection {
    /// 创建新的自定义段
    pub fn new(name: String, data: Vec<u8>) -> Self {
        let section_type = CustomSectionType::from_name(&name);
        Self { name, section_type, data }
    }

    /// 创建 DWARF 调试段
    pub fn new_dwarf_section(section_type: CustomSectionType, data: Vec<u8>) -> Self {
        let name = section_type.name().to_string();
        Self { name, section_type, data }
    }

    /// 创建外部调试信息段
    pub fn new_external_debug_info(url: String) -> Self {
        let data = url.as_bytes().to_vec();
        Self { name: "external_debug_info".to_string(), section_type: CustomSectionType::ExternalDebugInfo, data }
    }

    /// 创建源映射段
    pub fn new_source_mapping(url: String) -> Self {
        let data = url.as_bytes().to_vec();
        Self { name: "sourceMappingURL".to_string(), section_type: CustomSectionType::SourceMappingURL, data }
    }

    /// 获取段数据作为字符串（用于 URL 类型的段）
    pub fn data_as_string(&self) -> Result<String, std::str::Utf8Error> {
        std::str::from_utf8(&self.data).map(|s| s.to_string())
    }
}

impl DwarfCustomSections {
    /// 创建新的 DWARF 自定义段管理器
    pub fn new() -> Self {
        Self { sections: HashMap::new(), external_debug_info: None, source_mapping: None }
    }

    /// 添加自定义段
    pub fn add_section(&mut self, section: CustomSection) {
        match &section.section_type {
            CustomSectionType::ExternalDebugInfo => {
                if let Ok(url) = section.data_as_string() {
                    self.external_debug_info = Some(ExternalDebugInfo { url });
                }
            }
            CustomSectionType::SourceMappingURL => {
                if let Ok(url) = section.data_as_string() {
                    self.source_mapping = Some(SourceMapping { url });
                }
            }
            _ => {}
        }

        self.sections.insert(section.name.clone(), section);
    }

    /// 获取 DWARF 段
    pub fn get_dwarf_section(&self, section_type: CustomSectionType) -> Option<&CustomSection> {
        if section_type.is_dwarf_section() {
            self.sections.get(section_type.name())
        }
        else {
            None
        }
    }

    /// 获取所有 DWARF 段
    pub fn get_all_dwarf_sections(&self) -> Vec<&CustomSection> {
        self.sections.values().filter(|section| section.section_type.is_dwarf_section()).collect()
    }

    /// 是否包含嵌入的 DWARF 信息
    pub fn has_embedded_dwarf(&self) -> bool {
        self.sections.values().any(|section| section.section_type.is_dwarf_section())
    }

    /// 是否有外部调试信息
    pub fn has_external_debug_info(&self) -> bool {
        self.external_debug_info.is_some()
    }

    /// 获取外部调试信息 URL
    pub fn get_external_debug_url(&self) -> Option<&str> {
        self.external_debug_info.as_ref().map(|info| info.url.as_str())
    }

    /// 获取源映射 URL
    pub fn get_source_mapping_url(&self) -> Option<&str> {
        self.source_mapping.as_ref().map(|mapping| mapping.url.as_str())
    }

    /// 从 DWARF 信息创建自定义段
    pub fn from_dwarf_info(dwarf_info: &DwarfInfo) -> Self {
        let mut sections = Self::new();

        // 这里可以实现将 DwarfInfo 转换为二进制数据的逻辑
        // 目前只是占位符实现

        // 添加 .debug_info 段
        if !dwarf_info.compilation_units.is_empty() {
            let debug_info_data = vec![]; // TODO: 序列化编译单元
            let section = CustomSection::new_dwarf_section(CustomSectionType::DebugInfo, debug_info_data);
            sections.add_section(section);
        }

        // 添加 .debug_line 段
        if !dwarf_info.line_programs.is_empty() {
            let debug_line_data = vec![]; // TODO: 序列化行号程序
            let section = CustomSection::new_dwarf_section(CustomSectionType::DebugLine, debug_line_data);
            sections.add_section(section);
        }

        // 添加 .debug_str 段
        if !dwarf_info.string_table.strings.is_empty() {
            let section =
                CustomSection::new_dwarf_section(CustomSectionType::DebugStr, dwarf_info.string_table.strings.clone());
            sections.add_section(section);
        }

        sections
    }

    /// 移除所有 DWARF 段（用于外部调试信息）
    pub fn remove_dwarf_sections(&mut self) {
        self.sections.retain(|_, section| !section.section_type.is_dwarf_section());
    }
}

impl Default for DwarfCustomSections {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalDebugInfo {
    /// 创建新的外部调试信息引用
    pub fn new(url: String) -> Self {
        Self { url }
    }

    /// 是否为相对路径
    pub fn is_relative_url(&self) -> bool {
        !self.url.starts_with("http://") && !self.url.starts_with("https://") && !self.url.starts_with("file://")
    }

    /// 是否为文件 URL
    pub fn is_file_url(&self) -> bool {
        self.url.starts_with("file://")
    }
}

impl SourceMapping {
    /// 创建新的源映射信息
    pub fn new(url: String) -> Self {
        Self { url }
    }
}
