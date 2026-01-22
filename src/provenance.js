// ============== Constants ==============

// Built-in functions to skip when searching for definitions (using Set for O(1) lookup)
const BUILTIN_FUNCTIONS = new Set([
    'if', 'for', 'while', 'return', 'print', 'range', 'len', 'int', 'float', 'str',
    'list', 'dict', 'set', 'tuple', 'assert', 'isinstance', 'getattr', 'setattr',
    'empty_strided_cpu', 'empty_strided_cuda', 'reinterpret_tensor', 'as_strided',
    'assert_size_stride', 'rand_strided'
]);

// ============== Data Storage ==============

let preGradGraphData = null;
let postGradGraphData = null;
let codeData = null;
let cppCodeData = null;

let preToPost = {};
let postToPre = {};
let pyCodeToPost = {};
let postToPyCode = {};
let postToCppCode = {};
let cppCodeToPost = {};

let lineMappings = null;

let currentSelection = {
    editorId: null,
    lineNumber: null
};

let maximizedPanel = null;

// ============== Error Display ==============

/**
 * Shows an error banner at the top of the page.
 * @param {string} message - The error message to display
 */
function showErrorBanner(message) {
    const existing = document.querySelector('.error-banner');
    if (existing) existing.remove();

    const banner = document.createElement('div');
    banner.className = 'error-banner';
    banner.style.cssText = 'background: #ffebee; color: #c62828; padding: 8px 16px; border-bottom: 1px solid #ef9a9a; font-size: 13px;';
    banner.textContent = message;

    const container = document.querySelector('.editor-container');
    if (container && container.parentElement) {
        container.parentElement.insertBefore(banner, container);
    }
}

// ============== Sanitization ==============

/**
 * Sanitizes HTML from highlight.js to prevent XSS.
 * Only allows span tags with class attributes (which is what highlight.js produces).
 * @param {string} html - HTML string from highlight.js
 * @returns {string} Sanitized HTML string
 */
function sanitizeHighlightHtml(html) {
    // highlight.js only produces <span class="hljs-*">...</span> tags
    // Remove any other HTML tags or attributes that shouldn't be there
    return html
        // Remove script tags and their content
        .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
        // Remove event handlers
        .replace(/\s*on\w+\s*=\s*["'][^"']*["']/gi, '')
        // Remove javascript: URLs
        .replace(/javascript:/gi, '')
        // Remove data: URLs (except for safe ones)
        .replace(/data:(?!image\/(png|jpg|jpeg|gif|svg\+xml))[^"'\s]*/gi, '');
}

/**
 * Escapes special regex characters in a string.
 * @param {string} str - String to escape
 * @returns {string} Escaped string safe for use in RegExp
 */
function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// ============== Line Mappings ==============

/**
 * Initializes the line number mappings from the pre-processed JSON data.
 * Expects a JSON structure with: preToPost, postToPre, pyCodeToPost,
 * postToPyCode, cppCodeToPost, postToCppCode - each mapping source line
 * numbers to arrays of target line numbers.
 */
function initializeLineMappings() {
    try {
        const lineMappingsElement = document.getElementById('lineMappings');
        if (lineMappingsElement) {
            lineMappings = JSON.parse(lineMappingsElement.textContent);

            preToPost = lineMappings.preToPost || {};
            postToPre = lineMappings.postToPre || {};
            pyCodeToPost = lineMappings.pyCodeToPost || {};
            postToPyCode = lineMappings.postToPyCode || {};
            cppCodeToPost = lineMappings.cppCodeToPost || {};
            postToCppCode = lineMappings.postToCppCode || {};
        } else {
            console.warn('No line mappings element found - cross-panel highlighting will be disabled');
            showErrorBanner('Line mapping data not found. Cross-panel highlighting is disabled.');
        }
    } catch (error) {
        console.error('Error initializing line mappings:', error);
        showErrorBanner(`Failed to parse line mappings: ${error.message}. Cross-panel highlighting is disabled.`);
    }
}

/**
 * Splits HTML content by newlines while preserving HTML tags intact within each line.
 * Trims leading/trailing empty lines to ensure line numbers align with displayed content.
 * @param {string} html - HTML string to split
 * @returns {string[]} Array of HTML line strings
 */
function splitHtmlByLines(html) {
    if (!html) return [];

    let lines = html.split('\n');

    // Remove leading empty lines
    while (lines.length > 0 && lines[0].trim() === '') {
        lines.shift();
    }

    // Remove trailing empty lines
    while (lines.length > 0 && lines[lines.length - 1].trim() === '') {
        lines.pop();
    }

    return lines;
}

/**
 * Gets the total number of line mappings for a specific line in an editor.
 * @param {string} editorId - The editor ID ('preGradGraph', 'postGradGraph', or 'generatedCode')
 * @param {number} lineNum - The 1-based line number
 * @returns {number} The count of mappings, or 0 if none exist or editor is unknown
 */
function getMappingCount(editorId, lineNum) {
    switch (editorId) {
        case 'preGradGraph':
            return (preToPost[lineNum] || []).length;
        case 'postGradGraph':
            return (postToPre[lineNum] || []).length +
                   (postToPyCode[lineNum] || []).length +
                   (postToCppCode[lineNum] || []).length;
        case 'generatedCode':
            return (pyCodeToPost[lineNum] || []).length +
                   (cppCodeToPost[lineNum] || []).length;
        default:
            return 0;
    }
}

// ============== Editor Setup ==============

/**
 * Setup editor content with line numbers and event handlers.
 * Handles both plain text and HTML content (for syntax highlighting).
 * @param {string} editorId - The editor element ID
 * @param {string[]} lines - Array of line content strings
 * @param {boolean} isHighlighted - Whether lines contain HTML from syntax highlighting
 */
function setupEditorContent(editorId, lines, isHighlighted = false) {
    if (!lines) {
        console.warn(`setupEditorContent: No lines provided for ${editorId}`);
        return;
    }

    const editor = document.getElementById(editorId);
    if (!editor) {
        console.warn(`setupEditorContent: Editor element '${editorId}' not found`);
        return;
    }

    editor.innerHTML = '';

    lines.forEach((line, index) => {
        const lineDiv = document.createElement('div');
        lineDiv.className = 'line';
        lineDiv.dataset.lineNumber = index + 1;

        const lineNumber = document.createElement('span');
        lineNumber.className = 'line-number';
        lineNumber.textContent = index + 1;

        const lineContent = document.createElement('span');
        lineContent.className = 'line-content';

        // Use innerHTML for highlighted content (sanitized), textContent for plain text
        if (isHighlighted) {
            lineContent.innerHTML = sanitizeHighlightHtml(line);
        } else {
            lineContent.textContent = line;
        }

        // Check if this line has any matches (using consolidated logic)
        const lineNum = index + 1;
        const hasMatch = getMappingCount(editorId, lineNum) > 0;

        if (hasMatch) {
            lineContent.classList.add('has-match');
            lineDiv.classList.add('has-match');

            // Add mapping count badge if more than 1 mapping
            const count = getMappingCount(editorId, lineNum);
            if (count > 1) {
                const badge = document.createElement('span');
                badge.className = 'mapping-count';
                badge.textContent = count;
                lineDiv.appendChild(badge);
            }
        }

        lineDiv.appendChild(lineNumber);
        lineDiv.appendChild(lineContent);

        // Event handlers
        lineDiv.addEventListener('click', () => handleLineClick(editorId, index + 1));
        lineDiv.addEventListener('dblclick', (e) => handleDoubleClick(editorId, index + 1, e));
        lineDiv.addEventListener('mouseenter', () => handleLineHover(editorId, index + 1));
        lineDiv.addEventListener('mouseleave', clearHighlights);

        editor.appendChild(lineDiv);
    });

    // Update stats after content is set up
    updateEditorStats(editorId);
}

/**
 * Updates the stats display for an editor (line count and mapped count).
 * @param {string} editorId - The editor element ID
 */
function updateEditorStats(editorId) {
    const statsMap = {
        'preGradGraph': 'preGradStats',
        'postGradGraph': 'postGradStats',
        'generatedCode': 'generatedCodeStats'
    };

    const statsId = statsMap[editorId];
    if (!statsId) return;

    const stats = document.getElementById(statsId);
    const editor = document.getElementById(editorId);
    if (!stats || !editor) return;

    const lines = editor.querySelectorAll('.line');
    const mappedLines = editor.querySelectorAll('.line.has-match');

    stats.textContent = `${lines.length} lines, ${mappedLines.length} mapped`;
}

// ============== Line Highlighting ==============

/**
 * Handle line hover - highlights line and corresponding lines.
 * @param {string} editorId - The editor element ID
 * @param {number} lineNumber - The 1-based line number
 */
function handleLineHover(editorId, lineNumber) {
    clearHighlights();

    const hoveredLine = document.querySelector(`#${editorId} .line:nth-child(${lineNumber})`);
    if (hoveredLine) {
        hoveredLine.classList.add('highlight-source');
    }

    highlightCorrespondingLines(editorId, lineNumber);
}

/**
 * Clear all line highlights across all editors.
 */
function clearHighlights() {
    document.querySelectorAll('.line').forEach(line => {
        line.classList.remove('highlight', 'highlight-source', 'highlight-target');
    });
}

/**
 * Handle double-click - jump to function definition in generated code.
 * Searches for Python def, C++ function definitions, or async_compile assignments.
 * @param {string} editorId - The editor element ID
 * @param {number} lineNumber - The 1-based line number
 * @param {Event} event - The double-click event
 */
function handleDoubleClick(editorId, lineNumber, event) {
    // Only handle double-clicks in the generated code pane
    if (editorId !== 'generatedCode') return;

    const editor = document.getElementById(editorId);
    if (!editor) return;

    const clickedLine = editor.querySelector(`.line:nth-child(${lineNumber})`);
    if (!clickedLine) return;

    // Get text from line-content span, not the whole line (which includes line number)
    const lineContentSpan = clickedLine.querySelector('.line-content');
    if (!lineContentSpan) return;
    const lineText = lineContentSpan.textContent;

    // Extract function/kernel names from the line
    const funcCallMatch = lineText.match(/\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g);
    if (!funcCallMatch) return;

    // Get all function names called on this line
    const funcNames = funcCallMatch.map(m => m.replace(/\s*\($/, ''));

    // Search for definitions
    const lines = editor.querySelectorAll('.line');
    for (const funcName of funcNames) {
        // Skip built-in functions (O(1) lookup)
        if (BUILTIN_FUNCTIONS.has(funcName)) {
            continue;
        }

        // Escape regex special characters in function name
        const escapedName = escapeRegex(funcName);
        const defPattern = new RegExp(`\\b(def|void|static|inline)\\s+${escapedName}\\s*\\(`);
        const assignPattern = new RegExp(`^\\s*${escapedName}\\s*=\\s*(triton_|async_compile|extern_kernels)`);

        for (let i = 0; i < lines.length; i++) {
            const contentSpan = lines[i].querySelector('.line-content');
            if (!contentSpan) continue;
            const lineContent = contentSpan.textContent;

            if (defPattern.test(lineContent) || assignPattern.test(lineContent)) {
                // Found the definition - jump to it
                clearHighlights();
                lines[i].classList.add('highlight-source');
                lines[i].scrollIntoView({ behavior: 'smooth', block: 'center' });
                currentSelection = { editorId, lineNumber: i + 1 };
                event.preventDefault();
                event.stopPropagation();
                return;
            }
        }
    }
}

/**
 * Handle line click - highlights and sets current selection for keyboard nav.
 * @param {string} editorId - The editor element ID
 * @param {number} lineNumber - The 1-based line number
 */
function handleLineClick(editorId, lineNumber) {
    clearHighlights();

    const clickedLine = document.querySelector(`#${editorId} .line:nth-child(${lineNumber})`);
    if (clickedLine) {
        clickedLine.classList.add('highlight-source');
        clickedLine.scrollIntoView({
            behavior: 'smooth',
            block: 'center',
            inline: 'nearest'
        });
    }

    currentSelection = { editorId, lineNumber };
    highlightCorrespondingLines(editorId, lineNumber);
}

// ============== Syntax Highlighting ==============

/**
 * Highlight code using highlight.js and return the highlighted HTML.
 * Falls back to auto-detection if the specified language fails,
 * and escapes HTML if all highlighting attempts fail.
 * @param {string} code - Raw code to highlight
 * @param {string} language - Language identifier (e.g., 'python', 'cpp')
 * @returns {string} HTML string with syntax highlighting spans
 */
function highlightCode(code, language) {
    if (typeof hljs === 'undefined') {
        console.warn('highlight.js not loaded, using plain text');
        return escapeHtml(code);
    }
    try {
        const result = hljs.highlight(code, { language });
        return result.value;
    } catch (e) {
        console.warn(`Syntax highlighting failed for language '${language}': ${e.message}. Trying auto-detection.`);
        try {
            const result = hljs.highlightAuto(code);
            return result.value;
        } catch (e2) {
            console.error(`Syntax highlighting completely failed: ${e2.message}. Falling back to plain text.`);
            return escapeHtml(code);
        }
    }
}

/**
 * Escape HTML special characters to prevent injection.
 * @param {string} text - Text to escape
 * @returns {string} HTML-escaped string
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// ============== Initialization ==============

/**
 * Initialize data from pre-embedded content in the HTML.
 * Sets up all three editor panels with syntax-highlighted content.
 */
function initializeData() {
    try {
        const preGradGraph = document.querySelector('#preGradGraph pre code');
        const postGradGraph = document.querySelector('#postGradGraph pre code');
        const generatedCode = document.querySelector('#generatedCode pre code');

        // Initialize line mappings first
        initializeLineMappings();

        // Get raw text content and highlight with highlight.js
        if (preGradGraph) {
            const rawCode = preGradGraph.textContent;
            const highlighted = highlightCode(rawCode, 'python');
            preGradGraphData = splitHtmlByLines(highlighted);
        }
        if (postGradGraph) {
            const rawCode = postGradGraph.textContent;
            const highlighted = highlightCode(rawCode, 'python');
            postGradGraphData = splitHtmlByLines(highlighted);
        }
        if (generatedCode) {
            const rawCode = generatedCode.textContent;
            const isCpp = rawCode.includes('AOTInductorModel::run_impl');
            const language = isCpp ? 'cpp' : 'python';
            const highlighted = highlightCode(rawCode, language);

            if (isCpp) {
                cppCodeData = splitHtmlByLines(highlighted);
                codeData = null;
            } else {
                codeData = splitHtmlByLines(highlighted);
                cppCodeData = null;
            }
        }

        // Setup editors with highlighted content
        setupEditorContent('preGradGraph', preGradGraphData, true);
        setupEditorContent('postGradGraph', postGradGraphData, true);
        setupEditorContent('generatedCode', codeData || cppCodeData, true);

        // If it's C++ code, scroll to run_impl
        if (cppCodeData) {
            const cppEditor = document.getElementById('generatedCode');
            if (cppEditor) {
                const targetLine = Array.from(cppEditor.querySelectorAll('.line')).find(
                    line => line.textContent.includes('void AOTInductorModel::run_impl(')
                );
                if (targetLine) {
                    targetLine.scrollIntoView({ behavior: 'auto', block: 'center' });
                }
            }
        }
    } catch (error) {
        console.error('Error initializing data:', error);
        console.error(error.stack);

        // Show error to user
        const container = document.querySelector('.editor-container');
        if (container) {
            container.innerHTML = `
                <div style="padding: 20px; color: #c62828; background: #ffebee; margin: 10px; border-radius: 4px;">
                    <h3 style="margin-top: 0;">Failed to initialize provenance viewer</h3>
                    <p>Error: ${escapeHtml(error.message)}</p>
                    <p>This may be caused by corrupted data or browser issues. Check the console for details.</p>
                </div>
            `;
        }
    }
}

/**
 * Highlight corresponding lines in other editors based on line mappings.
 * @param {string} sourceEditorId - The source editor ID
 * @param {number} lineNumber - The 1-based line number in the source
 */
function highlightCorrespondingLines(sourceEditorId, lineNumber) {
    let correspondingLines = findCorrespondingLines(sourceEditorId, lineNumber);

    Object.entries(correspondingLines).forEach(([editorId, lines]) => {
        if (lines && editorId !== sourceEditorId) {
            const lineNumbers = Array.isArray(lines) ? lines : [lines];
            const middleIndex = Math.floor(lineNumbers.length / 2);
            let hasScrolled = false;

            lineNumbers.forEach((line, index) => {
                const lineElement = document.querySelector(`#${editorId} .line:nth-child(${line})`);
                if (lineElement) {
                    lineElement.classList.add('highlight-target');

                    if (index === middleIndex && !hasScrolled) {
                        lineElement.scrollIntoView({
                            behavior: 'smooth',
                            block: 'center',
                            inline: 'nearest'
                        });
                        hasScrolled = true;
                    }
                }
            });
        }
    });
}

/**
 * Find corresponding lines for a given source line based on mappings.
 * @param {string} sourceEditorId - The source editor ID
 * @param {number} lineNumber - The 1-based line number
 * @returns {Object} Object with editor IDs as keys and line number arrays as values
 */
function findCorrespondingLines(sourceEditorId, lineNumber) {
    let result = {};

    switch (sourceEditorId) {
        case 'preGradGraph':
            result.postGradGraph = preToPost[lineNumber] || [];
            if (result.postGradGraph.length > 0) {
                result.generatedCode = [];
                for (const postLine of result.postGradGraph) {
                    if (codeData) {
                        if (postToPyCode[postLine]) {
                            result.generatedCode.push(...postToPyCode[postLine]);
                        }
                    } else {
                        if (postToCppCode[postLine]) {
                            result.generatedCode.push(...postToCppCode[postLine]);
                        }
                    }
                }
            }
            break;

        case 'postGradGraph':
            result.preGradGraph = postToPre[lineNumber] || [];
            if (codeData) {
                result.generatedCode = postToPyCode[lineNumber] || [];
            } else {
                result.generatedCode = postToCppCode[lineNumber] || [];
            }
            break;

        case 'generatedCode':
            if (codeData) {
                result.postGradGraph = pyCodeToPost[lineNumber] || [];
            } else {
                result.postGradGraph = cppCodeToPost[lineNumber] || [];
            }
            if (result.postGradGraph.length > 0) {
                result.preGradGraph = [];
                for (const postLine of result.postGradGraph) {
                    if (postToPre[postLine]) {
                        result.preGradGraph.push(...postToPre[postLine]);
                    }
                }
            }
            break;
    }

    return result;
}

// ============== Keyboard Navigation ==============

/**
 * Navigate to the next or previous mapped line in the current editor.
 * @param {number} direction - 1 for next, -1 for previous
 */
function navigateToNextMappedLine(direction) {
    if (!currentSelection.editorId) {
        // If no selection, start from first mapped line
        const firstMapped = document.querySelector('.line.has-match');
        if (firstMapped) {
            const editorElement = firstMapped.closest('.editor');
            if (!editorElement || !editorElement.id) {
                console.warn('navigateToNextMappedLine: Could not find parent editor element');
                return;
            }
            const lineNum = parseInt(firstMapped.dataset.lineNumber, 10);
            if (isNaN(lineNum)) {
                console.warn('navigateToNextMappedLine: Invalid line number in dataset');
                return;
            }
            handleLineClick(editorElement.id, lineNum);
        }
        return;
    }

    const editor = document.getElementById(currentSelection.editorId);
    if (!editor) return;

    const lines = Array.from(editor.querySelectorAll('.line'));
    const startIndex = currentSelection.lineNumber - 1;

    // Search in the specified direction
    if (direction > 0) {
        for (let i = startIndex + 1; i < lines.length; i++) {
            if (lines[i].classList.contains('has-match')) {
                handleLineClick(currentSelection.editorId, i + 1);
                return;
            }
        }
        // Wrap around
        for (let i = 0; i < startIndex; i++) {
            if (lines[i].classList.contains('has-match')) {
                handleLineClick(currentSelection.editorId, i + 1);
                return;
            }
        }
    } else {
        for (let i = startIndex - 1; i >= 0; i--) {
            if (lines[i].classList.contains('has-match')) {
                handleLineClick(currentSelection.editorId, i + 1);
                return;
            }
        }
        // Wrap around
        for (let i = lines.length - 1; i > startIndex; i--) {
            if (lines[i].classList.contains('has-match')) {
                handleLineClick(currentSelection.editorId, i + 1);
                return;
            }
        }
    }
}

/**
 * Jump to corresponding line in the next/previous panel.
 * @param {number} direction - 1 for next panel, -1 for previous
 */
function jumpToCorrespondingPanel(direction) {
    if (!currentSelection.editorId || !currentSelection.lineNumber) return;

    const panels = ['preGradGraph', 'postGradGraph', 'generatedCode'];
    const currentIndex = panels.indexOf(currentSelection.editorId);
    const nextIndex = (currentIndex + direction + panels.length) % panels.length;

    const correspondingLines = findCorrespondingLines(currentSelection.editorId, currentSelection.lineNumber);
    const nextPanel = panels[nextIndex];

    if (correspondingLines[nextPanel] && correspondingLines[nextPanel].length > 0) {
        const targetLine = correspondingLines[nextPanel][0];
        handleLineClick(nextPanel, targetLine);
    } else {
        // No corresponding line, just focus the panel
        const editor = document.getElementById(nextPanel);
        if (editor) {
            const firstLine = editor.querySelector('.line');
            if (firstLine) {
                handleLineClick(nextPanel, 1);
            }
        }
    }
}

/**
 * Toggle help overlay visibility.
 */
function toggleHelp() {
    const help = document.getElementById('keyboard-help');
    if (help) {
        help.classList.toggle('visible');
    }
}

/**
 * Set up keyboard event handlers for navigation shortcuts.
 */
function setupKeyboardNavigation() {
    document.addEventListener('keydown', (e) => {
        // Ignore if typing in an input
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
            return;
        }

        switch (e.key) {
            case 'n':
                navigateToNextMappedLine(1);
                e.preventDefault();
                break;
            case 'N':
                navigateToNextMappedLine(-1);
                e.preventDefault();
                break;
            case 'j':
                jumpToCorrespondingPanel(1);
                e.preventDefault();
                break;
            case 'J':
                jumpToCorrespondingPanel(-1);
                e.preventDefault();
                break;
            case '?':
                toggleHelp();
                e.preventDefault();
                break;
            case 'Escape':
                clearHighlights();
                currentSelection = { editorId: null, lineNumber: null };
                // Also close help if open
                const help = document.getElementById('keyboard-help');
                if (help) help.classList.remove('visible');
                // Restore panels if maximized
                if (maximizedPanel) {
                    toggleMaximize(maximizedPanel);
                }
                e.preventDefault();
                break;
            case '1':
            case '2':
            case '3':
                const panelIndex = parseInt(e.key) - 1;
                const wrappers = document.querySelectorAll('.editor-wrapper');
                if (wrappers[panelIndex]) {
                    toggleMaximize(wrappers[panelIndex]);
                }
                e.preventDefault();
                break;
        }
    });
}

// ============== Resizable Panels ==============

/**
 * Set up drag-to-resize functionality for panel dividers.
 */
function setupResizablePanels() {
    const container = document.querySelector('.editor-container');
    const wrappers = document.querySelectorAll('.editor-wrapper');
    const divider1 = document.getElementById('divider1');
    const divider2 = document.getElementById('divider2');

    if (wrappers.length < 3 || !divider1 || !divider2) {
        console.warn('setupResizablePanels: Required elements missing', {
            wrapperCount: wrappers.length,
            hasDivider1: !!divider1,
            hasDivider2: !!divider2
        });
        return;
    }

    let isDragging = false;
    let dragDivider = null;

    function onMouseMove(e) {
        if (!isDragging || !dragDivider) return;

        const containerRect = container.getBoundingClientRect();

        if (dragDivider === divider1) {
            const newWidth = e.clientX - containerRect.left;
            const minWidth = 100;
            const maxWidth = containerRect.width - 200;

            if (newWidth > minWidth && newWidth < maxWidth) {
                wrappers[0].style.flex = `0 0 ${newWidth}px`;
            }
        } else if (dragDivider === divider2) {
            const firstWidth = wrappers[0].offsetWidth;
            const newWidth = e.clientX - containerRect.left - firstWidth - divider1.offsetWidth;
            const minWidth = 100;
            const maxWidth = containerRect.width - firstWidth - divider1.offsetWidth - 100;

            if (newWidth > minWidth && newWidth < maxWidth) {
                wrappers[1].style.flex = `0 0 ${newWidth}px`;
            }
        }
    }

    function onMouseUp() {
        isDragging = false;
        dragDivider = null;
        document.body.style.cursor = '';
        document.removeEventListener('mousemove', onMouseMove);
        document.removeEventListener('mouseup', onMouseUp);
    }

    [divider1, divider2].forEach(div => {
        div.addEventListener('mousedown', e => {
            isDragging = true;
            dragDivider = div;
            document.body.style.cursor = 'col-resize';
            document.addEventListener('mousemove', onMouseMove);
            document.addEventListener('mouseup', onMouseUp);
            e.preventDefault();
        });
    });
}

// ============== Panel Maximize ==============

/**
 * Set up double-click on headers to maximize/restore panels.
 */
function setupPanelMaximize() {
    const headers = document.querySelectorAll('.editor-header');
    headers.forEach(header => {
        header.style.cursor = 'pointer';
        header.addEventListener('dblclick', () => {
            const wrapper = header.parentElement;
            toggleMaximize(wrapper);
        });
    });
}

/**
 * Toggle maximize state for a panel.
 * @param {Element} wrapper - The editor-wrapper element to maximize/restore
 */
function toggleMaximize(wrapper) {
    const allWrappers = document.querySelectorAll('.editor-wrapper');
    const dividers = document.querySelectorAll('.divider');

    if (maximizedPanel === wrapper) {
        // Restore all panels
        allWrappers.forEach(w => {
            w.style.display = '';
            w.style.flex = '';
        });
        dividers.forEach(d => {
            d.style.display = '';
        });
        maximizedPanel = null;
    } else {
        // Maximize this panel, hide others
        allWrappers.forEach(w => {
            if (w === wrapper) {
                w.style.display = '';
                w.style.flex = '1';
            } else {
                w.style.display = 'none';
            }
        });
        dividers.forEach(d => {
            d.style.display = 'none';
        });
        maximizedPanel = wrapper;
    }
}

// ============== Entry Point ==============

window.addEventListener('DOMContentLoaded', () => {
    initializeData();
    setupResizablePanels();
    setupKeyboardNavigation();
    setupPanelMaximize();
});
