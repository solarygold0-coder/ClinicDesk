pub const NEXT_FILE_NO_KEY:&str="next_patient_file_no";
pub fn normalize_digits(input:&str)->String{input.chars().map(|c|match c{'٠'=>'0','١'=>'1','٢'=>'2','٣'=>'3','٤'=>'4','٥'=>'5','٦'=>'6','٧'=>'7','٨'=>'8','٩'=>'9',_=>c}).collect()}
#[cfg(test)]mod tests{use super::*;#[test]fn converts_arabic_indic_digits(){assert_eq!(normalize_digits("١٠٢٣"),"1023");}}
