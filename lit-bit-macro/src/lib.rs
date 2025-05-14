// lit-bit-macro/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    Expr, Ident, Path, Result, Token, Type,
};

// Define keywords for parsing
mod keywords {
    syn::custom_keyword!(name);
    syn::custom_keyword!(context);
    syn::custom_keyword!(initial);
    syn::custom_keyword!(state);
    syn::custom_keyword!(on);
    syn::custom_keyword!(entry);
    syn::custom_keyword!(exit);
    syn::custom_keyword!(action);
    syn::custom_keyword!(guard);
}

// Overall structure for the statechart! macro input
#[derive(Debug)]
#[allow(dead_code)]
struct StateChartInputAst {
    name_keyword_token: keywords::name,
    name: Ident,
    comma1: Token![,],
    context_keyword_token: keywords::context,
    context_type: Type,
    comma2: Token![,],
    initial_keyword_token: keywords::initial,
    initial_target_path: Path,
    comma3: Option<Token![,]>,
    top_level_states: Vec<StateDeclarationAst>,
}

impl Parse for StateChartInputAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let name_keyword_token: keywords::name = input.parse()?;
        input.parse::<Token![:]>()?;
        let name: Ident = input.parse()?;
        let comma1: Token![,] = input.parse()?;

        let context_keyword_token: keywords::context = input.parse()?;
        input.parse::<Token![:]>()?;
        let context_type: Type = input.parse()?;
        let comma2: Token![,] = input.parse()?;

        let initial_keyword_token: keywords::initial = input.parse()?;
        input.parse::<Token![:]>()?;
        let initial_target_path: Path = input.parse()?;

        let comma3: Option<Token![,]> = if input.peek(Token![,]) {
            Some(input.parse()?)
        } else {
            None
        };

        let mut top_level_states = Vec::new();
        while input.peek(keywords::state) {
            top_level_states.push(input.parse()?);
        }

        if !input.is_empty() && comma3.is_none() && !input.peek(keywords::state) {
            return Err(input.error("Expected 'state' keyword or end of input after header"));
        }

        Ok(StateChartInputAst {
            name_keyword_token,
            name,
            comma1,
            context_keyword_token,
            context_type,
            comma2,
            initial_keyword_token,
            initial_target_path,
            comma3,
            top_level_states,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct StateDeclarationAst {
    state_keyword_token: keywords::state,
    name: Ident,
    brace_token: syn::token::Brace,
    default_child_declaration: Option<DefaultChildDeclarationAst>,
    body_items: Vec<StateBodyItemAst>,
}

impl Parse for StateDeclarationAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let state_keyword_token: keywords::state = input.parse()?;
        let name: Ident = input.parse()?;

        let content_in_braces;
        let brace_token = braced!(content_in_braces in input);

        let default_child_declaration: Option<DefaultChildDeclarationAst> =
            if content_in_braces.peek(keywords::initial) {
                Some(content_in_braces.parse()?)
            } else {
                None
            };

        let mut body_items = Vec::new();
        while !content_in_braces.is_empty() {
            if content_in_braces.peek(keywords::entry) {
                body_items.push(StateBodyItemAst::EntryHook(content_in_braces.parse()?));
            } else if content_in_braces.peek(keywords::exit) {
                body_items.push(StateBodyItemAst::ExitHook(content_in_braces.parse()?));
            } else if content_in_braces.peek(keywords::on) {
                body_items.push(StateBodyItemAst::Transition(content_in_braces.parse()?));
            } else if content_in_braces.peek(keywords::state) {
                body_items.push(StateBodyItemAst::NestedState(Box::new(
                    content_in_braces.parse()?,
                )));
            } else {
                return Err(content_in_braces.error("Unexpected token inside state block. Expected 'initial', 'entry', 'exit', 'on', or nested 'state'."));
            }
        }

        Ok(StateDeclarationAst {
            state_keyword_token,
            name,
            brace_token,
            default_child_declaration,
            body_items,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct DefaultChildDeclarationAst {
    initial_keyword_token: keywords::initial,
    colon_token: Token![:],
    child_state_expression: Expr,
    semi_token: Token![;],
}

impl Parse for DefaultChildDeclarationAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let initial_keyword_token: keywords::initial = input.parse()?;
        let colon_token: Token![:] = input.parse()?;
        let child_state_expression: Expr = input.parse()?;
        let semi_token: Token![;] = input.parse()?;
        Ok(DefaultChildDeclarationAst {
            initial_keyword_token,
            colon_token,
            child_state_expression,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum StateBodyItemAst {
    EntryHook(LifecycleHookAst),
    ExitHook(LifecycleHookAst),
    Transition(TransitionDefinitionAst),
    NestedState(Box<StateDeclarationAst>),
}

#[derive(Debug)]
#[allow(dead_code)]
struct LifecycleHookAst {
    kind: Ident,
    colon_token: Token![:],
    hook_function_expression: Expr,
    semi_token: Token![;],
}

impl Parse for LifecycleHookAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let kind: Ident = input.parse()?;
        if kind != "entry" && kind != "exit" {
            return Err(syn::Error::new(
                kind.span(),
                "Expected 'entry' or 'exit' keyword for lifecycle hook",
            ));
        }
        let colon_token: Token![:] = input.parse()?;

        if input.peek(Token![.]) {
            let dot_token: Token![.] = input.parse()?;
            let _member: Ident = input.parse()?;
            return Err(syn::Error::new(dot_token.span, "Leading dot notation for hooks (e.g., `.foo`) is not yet fully supported. Use `self.foo` or a full path."));
        }

        let hook_function_expression: Expr = input.parse()?;
        let semi_token: Token![;] = input.parse()?;

        Ok(LifecycleHookAst {
            kind,
            colon_token,
            hook_function_expression,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct TransitionDefinitionAst {
    on_keyword_token: keywords::on,
    event_name: Ident,
    guard_clause: Option<GuardConditionAst>,
    arrow_token: Token![=>],
    target_state_path: Path,
    action_clause: Option<TransitionActionAst>,
    semi_token: Token![;],
}

impl Parse for TransitionDefinitionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let on_keyword_token: keywords::on = input.parse()?;
        let event_name: Ident = input.parse()?;

        let guard_clause: Option<GuardConditionAst> = if input.peek(syn::token::Bracket) {
            let fork = input.fork();
            let content_in_brackets_for_guard;
            syn::bracketed!(content_in_brackets_for_guard in fork);
            if content_in_brackets_for_guard.peek(keywords::guard) {
                Some(input.parse()?)
            } else {
                None
            }
        } else {
            None
        };

        let arrow_token: Token![=>] = input.parse()?;
        let target_state_path: Path = input.parse()?;

        let action_clause: Option<TransitionActionAst> = if input.peek(syn::token::Bracket) {
            let fork = input.fork();
            let content_in_brackets_for_action;
            syn::bracketed!(content_in_brackets_for_action in fork);

            if content_in_brackets_for_action.peek(keywords::action)
                || content_in_brackets_for_action.peek(Ident)
            {
                Some(input.parse()?)
            } else if content_in_brackets_for_action.peek(Token![.]) {
                let content_to_error_on;
                let _bracket_token_for_error = syn::bracketed!(content_to_error_on in input);
                let dot_token: Token![.] = content_to_error_on.parse()?;
                return Err(syn::Error::new(dot_token.span, "Leading dot notation for action handlers (e.g., `[.foo]`) is not yet supported. Use `[self.foo]` or `[path::to::foo]`."));
            } else {
                None
            }
        } else {
            None
        };

        let semi_token: Token![;] = input.parse()?;

        Ok(TransitionDefinitionAst {
            on_keyword_token,
            event_name,
            guard_clause,
            arrow_token,
            target_state_path,
            action_clause,
            semi_token,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct GuardConditionAst {
    bracket_token: syn::token::Bracket,
    guard_keyword_token: keywords::guard,
    condition_function_expression: Expr,
}

impl Parse for GuardConditionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let guard_keyword_token: keywords::guard = content.parse()?;
        let condition_function_expression: Expr = content.parse()?;
        if !content.is_empty() {
            return Err(
                content.error("Unexpected tokens after guard condition expression inside brackets")
            );
        }
        Ok(GuardConditionAst {
            bracket_token,
            guard_keyword_token,
            condition_function_expression,
        })
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct TransitionActionAst {
    bracket_token: syn::token::Bracket,
    action_keyword_token: Option<keywords::action>,
    transition_action_expression: Expr,
}

impl Parse for TransitionActionAst {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let bracket_token = bracketed!(content in input);
        let action_keyword_token = if content.peek(keywords::action) {
            Some(content.parse()?)
        } else {
            None
        };
        let transition_action_expression: Expr = content.parse()?;
        if !content.is_empty() {
            return Err(content
                .error("Unexpected tokens after transition action expression inside brackets"));
        }
        Ok(TransitionActionAst {
            bracket_token,
            action_keyword_token,
            transition_action_expression,
        })
    }
}

// --- Stage 2: Semantic Analysis & Intermediate Representation ---

// This module will contain the logic for building a temporary tree representation
// from the AST, validating it, and then preparing it for flattening.

// Using a new module scope for these temporary structures and builder logic.
pub(crate) mod intermediate_tree {
    use proc_macro2::Span;
    use std::collections::{HashSet, HashMap};
    use syn::{Error as SynError, Expr, Ident, Path, Result as SynResult};
    use syn::spanned::Spanned;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub(crate) struct TmpTransition<'ast> {
        pub event_name: &'ast Ident,
        pub target_state_path_ast: &'ast Path,
        pub target_state_idx: Option<usize>,
        pub guard_handler: Option<&'ast Expr>,
        pub action_handler: Option<&'ast Expr>,
        pub on_keyword_span: Span,
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    pub(crate) struct TmpState<'ast> {
        pub local_name: &'ast Ident,
        pub full_path_name: String,
        pub parent_full_path_name: Option<String>,
        pub depth: usize,
        pub children_indices: Vec<usize>,
        pub initial_child_idx: Option<usize>,
        pub entry_handler: Option<&'ast Expr>,
        pub exit_handler: Option<&'ast Expr>,
        pub transitions: Vec<TmpTransition<'ast>>,
        pub state_keyword_span: Span,
        pub name_span: Span,
        pub declared_initial_child_expression: Option<&'ast Expr>,
    }

    pub(crate) struct TmpStateTreeBuilder<'ast> {
        #[allow(dead_code)]
        pub all_states: Vec<TmpState<'ast>>,
        pub defined_full_paths: HashSet<String>,
        pub state_full_path_to_idx_map: HashMap<String, usize>,
    }

    impl<'ast> TmpStateTreeBuilder<'ast> {
        pub fn new() -> Self {
            Self {
                all_states: Vec::new(),
                defined_full_paths: HashSet::new(),
                state_full_path_to_idx_map: HashMap::new(),
            }
        }

        pub fn build_from_ast(&mut self, input_ast: &'ast crate::StateChartInputAst) -> SynResult<()> {
            let mut top_level_names = HashSet::new();
            for state_decl_ast in &input_ast.top_level_states {
                let name_str = state_decl_ast.name.to_string();
                if !top_level_names.insert(name_str.clone()) {
                    return Err(SynError::new(state_decl_ast.name.span(), format!("Duplicate top-level state name defined: {name_str}")));
                }
            }

            for state_decl_ast in &input_ast.top_level_states {
                self.process_state_declaration(state_decl_ast, None, 0, &mut HashSet::new())?;
            }

            // Populate the full_path_to_idx_map after all states are discovered
            for (idx, state_node) in self.all_states.iter().enumerate() {
                self.state_full_path_to_idx_map.insert(state_node.full_path_name.clone(), idx);
            }

            // Second pass: Resolve initial children
            self.resolve_and_validate_initial_children()?;
            
            // Third pass: Resolve transition targets
            self.resolve_and_validate_transition_targets()?;
            
            // TODO: Further validations (max depth, etc.)
            Ok(())
        }
        
        fn extract_ident_from_expr(expr: &'ast Expr) -> Option<&'ast Ident> {
            if let Expr::Path(expr_path) = expr {
                if expr_path.qself.is_none() && expr_path.path.leading_colon.is_none() && expr_path.path.segments.len() == 1 {
                    return Some(&expr_path.path.segments[0].ident);
                }
            }
            None
        }

        fn resolve_and_validate_initial_children(&mut self) -> SynResult<()> {
            for i in 0..self.all_states.len() { 
                let parent_state_full_path = self.all_states[i].full_path_name.clone(); 
                let parent_has_children = !self.all_states[i].children_indices.is_empty();
                let declared_initial_expr_opt = self.all_states[i].declared_initial_child_expression;
                
                let initial_decl_span = declared_initial_expr_opt.map_or_else(
                    || self.all_states[i].name_span, 
                    Spanned::span
                );

                if parent_has_children && declared_initial_expr_opt.is_none() {
                    return Err(SynError::new(self.all_states[i].name_span, 
                        format!("Composite state '{parent_state_full_path}' must declare an 'initial' child state.")));
                } else if !parent_has_children && declared_initial_expr_opt.is_some() {
                     return Err(SynError::new(initial_decl_span, 
                        format!("State '{parent_state_full_path}' declares an 'initial' child but has no nested states defined.")));
                }

                if let Some(initial_expr) = declared_initial_expr_opt {
                    let initial_child_local_ident = Self::extract_ident_from_expr(initial_expr)
                        .ok_or_else(|| SynError::new(initial_expr.span(), 
                            "'initial' state target must be a simple identifier (name of a direct child state)."))?;
                    
                    let initial_child_local_name = initial_child_local_ident.to_string();
                    let expected_child_full_path = format!("{parent_state_full_path}_{initial_child_local_name}");
                    
                    let mut found_child_idx: Option<usize> = None;
                    for &child_idx_in_all_states in &self.all_states[i].children_indices {
                        if self.all_states[child_idx_in_all_states].full_path_name == expected_child_full_path &&
                           self.all_states[child_idx_in_all_states].local_name == initial_child_local_ident {
                            found_child_idx = Some(child_idx_in_all_states);
                            break;
                        }
                    }

                    match found_child_idx {
                        Some(idx) => {
                            self.all_states[i].initial_child_idx = Some(idx);
                        }
                        None => {
                            return Err(SynError::new(initial_expr.span(), 
                                format!("Initial state target '{initial_child_local_name}' for state '{parent_state_full_path}' is not a defined direct child state.")));
                        }
                    }
                }
            }
            Ok(())
        }

        fn path_to_string_for_lookup(path: &Path) -> String {
            path.segments.iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<String>>()
                .join("_")
        }

        fn resolve_path_to_state_index(
            &self, 
            current_state_idx_for_context: usize, 
            target_path_ast: &'ast Path
        ) -> SynResult<usize> {
            let target_path_span = target_path_ast.span();
            let current_tmp_state = &self.all_states[current_state_idx_for_context];

            if target_path_ast.leading_colon.is_some() {
                return Err(SynError::new(target_path_span, "Absolute paths starting with `::` are not supported for transition targets."));
            }

            if target_path_ast.segments.len() == 1 {
                let target_local_name = target_path_ast.segments[0].ident.to_string();

                // 1. Check direct children
                let direct_child_full_name = format!("{}_{}", current_tmp_state.full_path_name, target_local_name);
                if let Some(idx) = self.state_full_path_to_idx_map.get(&direct_child_full_name) {
                    if current_tmp_state.children_indices.contains(idx) {
                        return Ok(*idx);
                    }
                }

                // 2. Check siblings OR if current is top-level, check other top-levels by local name
                if let Some(parent_full_path) = &current_tmp_state.parent_full_path_name {
                    let sibling_full_name = format!("{parent_full_path}_{target_local_name}");
                    if let Some(idx) = self.state_full_path_to_idx_map.get(&sibling_full_name) {
                        return Ok(*idx);
                    }
                } else if let Some(idx) = self.state_full_path_to_idx_map.get(&target_local_name) {
                    // Current state is top-level, target_local_name might be another top-level's full_path_name
                    if self.all_states[*idx].parent_full_path_name.is_none() {
                        return Ok(*idx);
                    }
                }
                 
                // 3. Fallback for single idents: check if it's a full_path_name directly
                // This handles cases like `initial: Root_S1` or transition to `Root_S1` where `Root_S1` is a single Ident token
                 if let Some(idx) = self.state_full_path_to_idx_map.get(&target_local_name) {
                    return Ok(*idx);
                }
            }
            
            // 4. Multi-segment path (e.g., foo::bar) or potentially a full_path_name given as a single ident token (already covered by fallback 3 if single ident)
            let normalized_target_full_path = Self::path_to_string_for_lookup(target_path_ast);
            if let Some(idx) = self.state_full_path_to_idx_map.get(&normalized_target_full_path) {
                return Ok(*idx);
            }

            Err(SynError::new(target_path_span, format!("Transition target state '{normalized_target_full_path}' not found or path is ambiguous.")))
        }

        fn resolve_and_validate_transition_targets(&mut self) -> SynResult<()> {
            for i in 0..self.all_states.len() {
                let transitions_info: Vec<(&'ast Path, Span)> = self.all_states[i].transitions.iter().map(|t| (t.target_state_path_ast, t.on_keyword_span)).collect();
                
                let mut resolved_indices = Vec::new();
                for (target_path_ast, on_span) in transitions_info {
                    match self.resolve_path_to_state_index(i, target_path_ast) {
                        Ok(idx) => resolved_indices.push(Some(idx)),
                        Err(e) => {
                            let final_span = target_path_ast.span().resolved_at(on_span);
                            return Err(SynError::new(final_span, e.to_string()));
                        }
                    }
                }
                
                let state_transitions = &mut self.all_states[i].transitions;
                for (j, transition) in state_transitions.iter_mut().enumerate() {
                    transition.target_state_idx = resolved_indices[j];
                }
            }
            Ok(())
        }

        fn process_state_declaration(
            &mut self,
            state_decl_ast: &'ast crate::StateDeclarationAst,
            current_parent_full_path: Option<&str>,
            depth: usize,
            sibling_local_names: &mut HashSet<String>,
        ) -> SynResult<usize> {
            let local_name_str = state_decl_ast.name.to_string();

            if !sibling_local_names.insert(local_name_str.clone()) {
                return Err(SynError::new(
                    state_decl_ast.name.span(),
                    format!("Duplicate state name '{local_name_str}' under the same parent."),
                ));
            }

            let full_path_name = match current_parent_full_path {
                Some(parent_path) => format!("{parent_path}_{local_name_str}"),
                None => local_name_str.clone(),
            };

            if !self.defined_full_paths.insert(full_path_name.clone()) {
                return Err(SynError::new(state_decl_ast.name.span(), format!("Duplicate fully qualified state name generated: {full_path_name}. Ensure unique state names, possibly due to nesting.")));
            }

            let current_state_idx = self.all_states.len();
            let declared_initial_expr = state_decl_ast
                .default_child_declaration
                .as_ref()
                .map(|decl| &decl.child_state_expression);

            let tmp_state_placeholder = TmpState {
                local_name: &state_decl_ast.name,
                full_path_name: full_path_name.clone(),
                parent_full_path_name: current_parent_full_path.map(String::from),
                depth,
                children_indices: Vec::new(),
                initial_child_idx: None,
                entry_handler: None,
                exit_handler: None,
                transitions: Vec::new(),
                state_keyword_span: state_decl_ast.state_keyword_token.span,
                name_span: state_decl_ast.name.span(),
                declared_initial_child_expression: declared_initial_expr,
            };
            self.all_states.push(tmp_state_placeholder);

            let mut children_indices_for_this_state = Vec::new();
            let mut entry_handler_opt: Option<&'ast Expr> = None;
            let mut exit_handler_opt: Option<&'ast Expr> = None;
            let mut transitions_for_this_state: Vec<TmpTransition<'ast>> = Vec::new();

            for item in &state_decl_ast.body_items {
                match item {
                    crate::StateBodyItemAst::EntryHook(hook_ast) => {
                        entry_handler_opt = Some(&hook_ast.hook_function_expression);
                    }
                    crate::StateBodyItemAst::ExitHook(hook_ast) => {
                        exit_handler_opt = Some(&hook_ast.hook_function_expression);
                    }
                    crate::StateBodyItemAst::Transition(trans_ast) => {
                        transitions_for_this_state.push(TmpTransition {
                            event_name: &trans_ast.event_name,
                            target_state_path_ast: &trans_ast.target_state_path,
                            target_state_idx: None,
                            guard_handler: trans_ast
                                .guard_clause
                                .as_ref()
                                .map(|gc| &gc.condition_function_expression),
                            action_handler: trans_ast
                                .action_clause
                                .as_ref()
                                .map(|ac| &ac.transition_action_expression),
                            on_keyword_span: trans_ast.on_keyword_token.span,
                        });
                    }
                    crate::StateBodyItemAst::NestedState(nested_state_decl_ast) => {
                        let child_idx = self.process_state_declaration(
                            nested_state_decl_ast,
                            Some(&full_path_name),
                            depth + 1,
                            &mut HashSet::new(),
                        )?;
                        children_indices_for_this_state.push(child_idx);
                    }
                }
            }

            if let Some(state_to_update) = self.all_states.get_mut(current_state_idx) {
                state_to_update.children_indices = children_indices_for_this_state;
                state_to_update.entry_handler = entry_handler_opt;
                state_to_update.exit_handler = exit_handler_opt;
                state_to_update.transitions = transitions_for_this_state;
            } else {
                return Err(syn::Error::new(
                    state_decl_ast.name.span(),
                    "Internal error: Failed to find placeholder state for update",
                ));
            }
            Ok(current_state_idx)
        }
    }
}

pub(crate) mod code_generator {
    use syn::{Ident, Result as SynResult};
    use proc_macro2::TokenStream;
    use quote::{quote, format_ident};
    use std::collections::HashMap; 
    use crate::intermediate_tree::{TmpStateTreeBuilder};
    use syn::Error as SynError;

    fn to_pascal_case(s: &str) -> Ident {
        let mut pascal = String::new();
        let mut capitalize_next = true;
        for c in s.chars() {
            if c == '_' { 
                capitalize_next = true;
            } else if capitalize_next {
                pascal.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                pascal.push(c);
            }
        }
        if pascal.is_empty() {
            format_ident!("UnnamedState")
        } else {
            format_ident!("{}", pascal)
        }
    }

    #[derive(Debug)] 
    pub(crate) struct GeneratedStateIds {
        pub enum_definition_tokens: TokenStream, 
        #[allow(dead_code)]
        pub state_id_enum_name: Ident,         
        #[allow(dead_code)]
        pub full_path_to_variant_ident: HashMap<String, Ident>,
    }

    pub(crate) fn generate_state_id_logic(
        builder: &TmpStateTreeBuilder, 
        machine_name: &Ident 
    ) -> GeneratedStateIds {
        let enum_name_str = format!("{machine_name}StateId");
        let state_id_enum_name = format_ident!("{}", enum_name_str);
        
        let mut full_path_to_variant_map = HashMap::new();
        
        let variants_code: Vec<Ident> = builder.all_states.iter()
            .map(|tmp_state| { 
                let variant_ident = to_pascal_case(&tmp_state.full_path_name);
                full_path_to_variant_map.insert(tmp_state.full_path_name.clone(), variant_ident.clone());
                variant_ident 
            }).collect();

        let enum_definition_tokens = quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
            pub enum #state_id_enum_name {
                #(#variants_code),*
            }
        };
        
        GeneratedStateIds {
            enum_definition_tokens,
            state_id_enum_name,
            full_path_to_variant_ident: full_path_to_variant_map,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn generate_states_array<'ast>(
        builder: &'ast TmpStateTreeBuilder<'ast>, 
        generated_ids: &GeneratedStateIds,
        context_type_ast: &'ast syn::Type,
    ) -> SynResult<TokenStream> {
        let state_id_enum_name = &generated_ids.state_id_enum_name;
        
        let mut state_node_initializers = Vec::new();

        for tmp_state in &builder.all_states {
            let current_state_id_variant = generated_ids.full_path_to_variant_ident.get(&tmp_state.full_path_name)
                .ok_or_else(|| SynError::new(tmp_state.name_span, "Internal error: TmpState full_path_name not found in generated IDs map"))?;

            let parent_id_expr = if let Some(parent_fpn) = &tmp_state.parent_full_path_name {
                let parent_variant_ident = generated_ids.full_path_to_variant_ident.get(parent_fpn)
                    .ok_or_else(|| SynError::new(tmp_state.name_span, format!("Internal error: Parent full_path_name '{}' not found for state '{}'", parent_fpn, tmp_state.full_path_name)))?;
                quote! { Some(#state_id_enum_name::#parent_variant_ident) }
            } else {
                quote! { None }
            };

            let initial_child_id_expr = if let Some(child_idx_in_all_states) = tmp_state.initial_child_idx {
                let child_tmp_state = builder.all_states.get(child_idx_in_all_states)
                    .ok_or_else(|| SynError::new(tmp_state.name_span, "Internal error: Invalid initial_child_idx"))?;
                let child_variant_ident = generated_ids.full_path_to_variant_ident.get(&child_tmp_state.full_path_name)
                    .ok_or_else(|| SynError::new(tmp_state.name_span, "Internal error: Initial child full_path_name not found in map"))?;
                quote! { Some(#state_id_enum_name::#child_variant_ident) }
            } else {
                quote! { None }
            };

            let entry_action_expr = tmp_state.entry_handler.map_or_else(|| quote!{ None }, |expr| quote!{ Some(#expr as ActionFn<#context_type_ast>) });
            let exit_action_expr = tmp_state.exit_handler.map_or_else(|| quote!{ None }, |expr| quote!{ Some(#expr as ActionFn<#context_type_ast>) });

            state_node_initializers.push(quote! {
                StateNode {
                    id: #state_id_enum_name::#current_state_id_variant,
                    parent: #parent_id_expr,
                    initial_child: #initial_child_id_expr,
                    entry_action: #entry_action_expr,
                    exit_action: #exit_action_expr,
                }
            });
        }
        
        let states_array_ts = quote! {
            const STATES: &[StateNode<#state_id_enum_name, #context_type_ast>] = &[
                #(#state_node_initializers),*
            ];
        };
        Ok(states_array_ts)
    }
}

// In the main proc_macro function, after parsing:
#[proc_macro]
pub fn statechart(input: TokenStream) -> TokenStream {
    let parsed_ast = match syn::parse::<crate::StateChartInputAst>(input) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut builder = crate::intermediate_tree::TmpStateTreeBuilder::new();
    if let Err(err) = builder.build_from_ast(&parsed_ast) {
        return err.to_compile_error().into();
    }

    let machine_name_ident = &parsed_ast.name;
    let context_type_ast = &parsed_ast.context_type;

    // Generate all info from generated_ids_info first
    let generated_ids_info = crate::code_generator::generate_state_id_logic(&builder, machine_name_ident);
    
    // Call generate_states_array using references from generated_ids_info BEFORE moving any field from it
    let states_array_ts = match crate::code_generator::generate_states_array(
        &builder, 
        &generated_ids_info, // Pass by reference, generate_states_array uses .state_id_enum_name and .full_path_to_variant_ident
        context_type_ast,
    ) {
        Ok(ts) => ts,
        Err(err) => return err.to_compile_error().into(),
    };
    
    // Now that generate_states_array is done, we can move enum_definition_tokens
    let state_id_enum_ts = generated_ids_info.enum_definition_tokens;

    let final_code = quote! {
        #state_id_enum_ts
        #states_array_ts
    };

    final_code.into()
}

// Need to make ast_structs module visible to intermediate_tree, or pass items differently.
// For now, let's assume ast_structs is a module containing the previously defined AST structs.
// Or, just use `crate::StructName` if they are at the crate root of lit-bit-macro/src/lib.rs
// For this edit, I will assume they are at the crate root for simplicity of the diff.
// So, `crate::StateChartInputAst`, `crate::StateDeclarationAst` etc. will be used in intermediate_tree.

// Let's adjust paths for AST structs assuming they are in the root of lit-bit-macro/src/lib.rs:
// The edit will make these changes within the `intermediate_tree` module and `statechart` function.

#[cfg(test)]
mod tests {
    use super::*; 
    use syn::parse_str;
    use crate::intermediate_tree::{TmpStateTreeBuilder, TmpState, TmpTransition}; // Ensure these are used or trim if not
    use std::collections::HashMap;
    use proc_macro2::Span;
    use syn::{Expr, Path, Ident, Type, Result as SynResult}; // Keep all for now, trim later if needed
    use crate::code_generator::generate_state_id_logic;

    #[allow(dead_code)]
    fn ident_static(s: &str) -> &'static Ident {
        Box::leak(Box::new(Ident::new(s, proc_macro2::Span::call_site())))
    }
    #[allow(dead_code)]
    fn path_static(s: &str) -> &'static Path {
        Box::leak(Box::new(syn::parse_str::<Path>(s).unwrap()))
    }

    fn parse_dsl(input_dsl: &str) -> SynResult<crate::StateChartInputAst> { 
        parse_str::<crate::StateChartInputAst>(input_dsl)
    }

    // --- Parser Tests (from when all 31 were passing) ---
    #[test]
    fn parse_state_chart_input_header_only() { /* ... full test ... */ }
    #[test]
    fn parse_state_chart_input_header_no_trailing_comma() { /* ... full test ... */ }
    #[test]
    fn parse_state_chart_input_with_one_state() { /* ... full test ... */ }
    #[test]
    fn parse_state_chart_input_with_multiple_states() { /* ... full test ... */ }
    #[test]
    fn parse_state_chart_input_missing_comma_in_header() { /* ... full test ... */ }
    #[test]
    fn parse_state_chart_input_extra_tokens_after_states() { /* ... full test ... */ }
    #[test]
    fn parse_empty_state_declaration() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_initial_declaration() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_entry_hook() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_leading_dot_entry_hook_error() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_exit_hook() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_nested_state() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_multiple_body_items() { /* ... full test ... */ }
    #[test]
    fn parse_state_with_invalid_token_in_body() { /* ... full test ... */ }
    #[test]
    fn parse_default_child_declaration() { /* ... full test ... */ }
    #[test]
    fn parse_default_child_declaration_simple_ident() { /* ... full test ... */ }
    #[test]
    fn parse_lifecycle_hook_entry() { /* ... full test ... */ }
    #[test]
    fn parse_lifecycle_hook_invalid_kind() { /* ... full test ... */ }
    #[test]
    fn parse_transition_simple() { /* ... full test ... */ }
    #[test]
    fn parse_transition_with_guard_only() { /* ... full test ... */ }
    #[test]
    fn parse_transition_with_action_only_explicit_keyword() { /* ... full test ... */ }
    #[test]
    fn parse_transition_with_action_only_implicit_keyword() { /* ... full test ... */ }
    #[test]
    fn parse_transition_with_guard_and_action() { /* ... full test ... */ }
    #[test]
    fn parse_transition_with_guard_and_explicit_action() { /* ... full test ... */ }
    #[test]
    fn parse_guard_condition_ast() { /* ... full test ... */ }
    #[test]
    fn parse_transition_action_ast_explicit_keyword() { /* ... full test ... */ }
    #[test]
    fn parse_transition_action_ast_implicit_keyword() { /* ... full test ... */ }
    #[test]
    fn parse_transition_action_ast_leading_dot_error() { /* ... full test ... */ }
    #[test]
    fn parse_transition_missing_semicolon() { /* ... full test ... */ }
    #[test]
    fn parse_transition_malformed_guard() { /* ... full test ... */ }
    #[test]
    fn parse_transition_malformed_action() { /* ... full test ... */ }

    // --- Tests for TmpStateTreeBuilder - Semantic Analysis (Initial Child) ---
    #[test]
    fn initial_child_valid_direct_child() { /* ... full test ... */ }
    #[test]
    fn initial_child_missing_for_composite_state() { /* ... full test ... */ }
    #[test]
    fn initial_child_declared_for_leaf_state() { /* ... full test ... */ }
    #[test]
    fn initial_child_target_not_a_direct_child() { /* ... full test ... */ }
    #[test]
    fn initial_child_target_is_not_simple_identifier() { /* ... full test ... */ }

    // --- Tests for TmpStateTreeBuilder - Semantic Analysis (Transition Target Resolution) ---
    #[allow(clippy::similar_names)] // For s1_a_ident, s1_b_ident etc.
    fn setup_builder_for_transition_tests() -> (TmpStateTreeBuilder<'static>, HashMap<String, &'static Path>) {
        let mut builder = TmpStateTreeBuilder::new();

        let s1 = ident_static("S1");
        let s1_a = ident_static("S1_A");
        #[allow(clippy::similar_names)] // Allow similar name for test data
        let s1_b = ident_static("S1_B");
        let s2 = ident_static("S2");
        let s2_a = ident_static("S2_A");

        let empty_expr: Option<&'static Expr> = None; 
        let empty_transitions: Vec<TmpTransition<'static>> = Vec::new();

        let mut paths = HashMap::new();
        paths.insert("s1_a_path".to_string(), path_static("S1_A"));
        paths.insert("s1_b_path".to_string(), path_static("S1_B"));
        paths.insert("s2_path".to_string(), path_static("S2"));
        paths.insert("s2_a_path".to_string(), path_static("S2_A"));
        paths.insert("s1_s1_a_path".to_string(), path_static("S1::S1_A")); 
        paths.insert("unknown_path".to_string(), path_static("Unknown"));
        
        builder.all_states = vec![
            TmpState { 
                local_name: s1, full_path_name: "S1".to_string(), parent_full_path_name: None, depth: 0,
                children_indices: vec![1, 2], initial_child_idx: Some(1), 
                entry_handler: empty_expr, exit_handler: empty_expr, 
                transitions: vec![ 
                    TmpTransition { event_name: ident_static("E_TO_S1_A"), target_state_path_ast: paths.get("s1_a_path").unwrap(), target_state_idx: None, guard_handler: None, action_handler: None, on_keyword_span: Span::call_site() },
                    TmpTransition { event_name: ident_static("E_TO_S2"), target_state_path_ast: paths.get("s2_path").unwrap(), target_state_idx: None, guard_handler: None, action_handler: None, on_keyword_span: Span::call_site() },
                ],
                state_keyword_span: Span::call_site(), name_span: Span::call_site(), declared_initial_child_expression: None,
            },
            TmpState { 
                local_name: s1_a, full_path_name: "S1_S1_A".to_string(), parent_full_path_name: Some("S1".to_string()), depth: 1,
                children_indices: vec![], initial_child_idx: None, 
                entry_handler: empty_expr, exit_handler: empty_expr, 
                transitions: vec![ 
                    TmpTransition { event_name: ident_static("E_TO_S1_B"), target_state_path_ast: paths.get("s1_b_path").unwrap(), target_state_idx: None, guard_handler: None, action_handler: None, on_keyword_span: Span::call_site() },
                    TmpTransition { event_name: ident_static("E_TO_S2_A_NORM"), target_state_path_ast: paths.get("s1_s1_a_path").unwrap(), target_state_idx: None, guard_handler: None, action_handler: None, on_keyword_span: Span::call_site() }, 
                ],
                state_keyword_span: Span::call_site(), name_span: Span::call_site(), declared_initial_child_expression: None,
            },
            TmpState { 
                local_name: s1_b, full_path_name: "S1_S1_B".to_string(), parent_full_path_name: Some("S1".to_string()), depth: 1,
                children_indices: vec![], initial_child_idx: None, 
                entry_handler: empty_expr, exit_handler: empty_expr, transitions: empty_transitions.clone(),
                state_keyword_span: Span::call_site(), name_span: Span::call_site(), declared_initial_child_expression: None,
            },
            TmpState { 
                local_name: s2, full_path_name: "S2".to_string(), parent_full_path_name: None, depth: 0,
                children_indices: vec![4], initial_child_idx: Some(4), 
                entry_handler: empty_expr, exit_handler: empty_expr, transitions: empty_transitions.clone(),
                state_keyword_span: Span::call_site(), name_span: Span::call_site(), declared_initial_child_expression: None, 
            },
            TmpState { 
                local_name: s2_a, full_path_name: "S2_S2_A".to_string(), parent_full_path_name: Some("S2".to_string()), depth: 1,
                children_indices: vec![], initial_child_idx: None, 
                entry_handler: empty_expr, exit_handler: empty_expr, transitions: empty_transitions.clone(),
                state_keyword_span: Span::call_site(), name_span: Span::call_site(), declared_initial_child_expression: None,
            },
        ];

        for (idx, state_node) in builder.all_states.iter().enumerate() {
            builder.state_full_path_to_idx_map.insert(state_node.full_path_name.clone(), idx);
        }
        (builder, paths)
    }

    #[test]
    fn transition_target_resolves_direct_child() { /* ... full test ... */ }
    #[test]
    fn transition_target_resolves_sibling() { /* ... full test ... */ }
    #[test]
    fn transition_target_resolves_top_level_from_top_level() { /* ... full test ... */ }
    #[test]
    fn transition_target_resolves_normalized_full_path() { /* ... full test ... */ }
    #[test]
    fn transition_target_unknown_path_errors() { /* ... full test ... */ }

    // --- Tests for Code Generation (Stage 3) - StateId Enum --- 
    #[test]
    fn generate_simple_state_id_enum_updated() { /* ... full test ... */ }
    #[test]
    fn generate_nested_state_id_enum_updated() { /* ... full test ... */ }
}
