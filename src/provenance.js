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

/**
 * Initializes the line number mappings from the pre-processed data.
 *
 * This function expects the line mappings to be already converted from node mappings
 * to line number mappings by the Rust backend. The mappings should contain:
 * - preToPost: {sourceLineNum: [targetLineNums]}
 * - postToPre: {sourceLineNum: [targetLineNums]}
 * - pyCodeToPost: {sourceLineNum: [targetLineNums]}
 * - postToPyCode: {sourceLineNum: [targetLineNums]}
 * - cppCodeToPost: {sourceLineNum: [targetLineNums]}
 * - postToCppCode: {sourceLineNum: [targetLineNums]}
 *
 * These mappings enable the UI to highlight corresponding lines
 * across different views when a user clicks on a line.
 */
function initializeLineMappings() {
    try {
        // Get the line mappings from the embedded JSON data
        const lineMappingsElement = document.getElementById('lineMappings');
        if (lineMappingsElement) {
            lineMappings = JSON.parse(lineMappingsElement.textContent);

            // Update global variables with the line mappings
            preToPost = lineMappings.preToPost || {};
            postToPre = lineMappings.postToPre || {};
            pyCodeToPost = lineMappings.pyCodeToPost || {};
            postToPyCode = lineMappings.postToPyCode || {};
            cppCodeToPost = lineMappings.cppCodeToPost || {};
            postToCppCode = lineMappings.postToCppCode || {};

            console.log('Line mappings initialized:', {
                preToPost,
                postToPre,
                pyCodeToPost,
                postToPyCode,
                cppCodeToPost,
                postToCppCode
            });
        } else {
            console.warn('No line mappings element found');
        }
    } catch (error) {
        console.error('Error initializing line mappings:', error);
    }
}


// Setup editor content
function setupEditorContent(editorId, lines) {
    if (!lines) return;

    const editor = document.getElementById(editorId);
    if (!editor) return;

    editor.innerHTML = '';  // Clear existing content

    lines.forEach((line, index) => {
        const lineDiv = document.createElement('div');
        lineDiv.className = 'line';

        // Create text nodes instead of using innerHTML
        const lineNumber = document.createElement('span');
        lineNumber.className = 'line-number';
        lineNumber.textContent = index + 1;

        const lineContent = document.createElement('span');
        lineContent.className = 'line-content';
        lineContent.textContent = line;

        // Check if this line has any matches
        const lineNum = index + 1;
        let hasMatch = false;
        switch (editorId) {
            case 'preGradGraph':
                hasMatch = preToPost[lineNum] && preToPost[lineNum].length > 0;
                break;
            case 'postGradGraph':
                hasMatch = (postToPre[lineNum] && postToPre[lineNum].length > 0) ||
                          (postToPyCode[lineNum] && postToPyCode[lineNum].length > 0) ||
                          (postToCppCode[lineNum] && postToCppCode[lineNum].length > 0);
                break;
            case 'generatedCode':
                hasMatch = (pyCodeToPost[lineNum] && pyCodeToPost[lineNum].length > 0) ||
                (cppCodeToPost[lineNum] && cppCodeToPost[lineNum].length > 0);
                break;
        }

        if (hasMatch) {
            lineContent.classList.add('has-match');
        }

        lineDiv.appendChild(lineNumber);
        lineDiv.appendChild(lineContent);

        // Add both click and hover handlers
        lineDiv.addEventListener('click', () => handleLineClick(editorId, index + 1));
        lineDiv.addEventListener('mouseenter', () => handleLineHover(editorId, index + 1));
        lineDiv.addEventListener('mouseleave', clearHighlights);

        editor.appendChild(lineDiv);
    });
}

function hasSelectedLines() {
    const selectedLines = document.querySelectorAll('.selected');
    return selectedLines && selectedLines.length > 0;
}

// Handle line hover
function handleLineHover(editorId, lineNumber) {
    if (hasSelectedLines()) {
        return; // Don't highlight if there are selected lines
    }

    // Clear previous highlights
    clearHighlights();

    // Add highlight to hovered line
    const hoveredLine = document.querySelector(`#${editorId} .line:nth-child(${lineNumber})`);
    if (hoveredLine) {
        hoveredLine.classList.add('highlight');
        // Remove scrolling for hovered panel
    }

    // Highlight and scroll corresponding lines
    UpdateCorrespondingLines(editorId, lineNumber, 'highlight');
}

// Clear all highlights
function clearHighlights() {
    document.querySelectorAll('.line').forEach(line => {
        line.classList.remove('highlight');
    });
}

// Clear all selected
function clearSelectedLines() {
    document.querySelectorAll('.line').forEach(line => {
        line.classList.remove('selected');
    });
}

function handleLineClick(editorId, lineNumber) {
    const clickedLine = document.querySelector(`#${editorId} .line:nth-child(${lineNumber})`);

    if (clickedLine && clickedLine.classList.contains('selected')) {
        clearSelectedLines();
        return;
    }

    if (clickedLine) {
        clickedLine.classList.add('selected');
        clickedLine.scrollIntoView({
            behavior: 'smooth',
            block: 'center',
            inline: 'nearest'
        });
    }

    // Select corresponding lines
    UpdateCorrespondingLines(editorId, lineNumber, 'selected');
}

// Initialize data from pre-embedded content
function initializeData() {
    try {
        // Get content from pre tags
        const preGradGraph = document.querySelector('#preGradGraph pre');
        const postGradGraph = document.querySelector('#postGradGraph pre');
        const generatedCode = document.querySelector('#generatedCode pre');

        if (preGradGraph) preGradGraphData = preGradGraph.textContent.split('\n');
        if (postGradGraph) postGradGraphData = postGradGraph.textContent.split('\n');
        if (generatedCode) {
            const content = generatedCode.textContent;
            if (content.includes('AOTInductorModel::run_impl')) {
                // This is C++ code
                cppCodeData = content.split('\n');
                codeData = null;
            } else {
                // This is Python code
                codeData = content.split('\n');
                cppCodeData = null;
            }
        }

        // Initialize line mappings from pre-processed data
        initializeLineMappings();

        // Setup highlighting
        setupEditorContent('preGradGraph', preGradGraphData);
        setupEditorContent('postGradGraph', postGradGraphData);
        setupEditorContent('generatedCode', codeData || cppCodeData);

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
    }
}

// Call initialization when the page loads
window.addEventListener('DOMContentLoaded', initializeData);

// Update corresponding lines to either highlight or selected state
function UpdateCorrespondingLines(sourceEditorId, lineNumber, newState) {
    let correspondingLines = findCorrespondingLines(sourceEditorId, lineNumber);

    Object.entries(correspondingLines).forEach(([editorId, lines]) => {
        // Skip scrolling if this is the source editor
        if (lines && editorId !== sourceEditorId) {
            // Handle both single numbers and arrays of numbers
            const lineNumbers = Array.isArray(lines) ? lines : [lines];

            // Get the middle line number for scrolling
            const middleIndex = Math.floor(lineNumbers.length / 2);
            let hasScrolled = false;

            lineNumbers.forEach((line, index) => {
                const lineElement = document.querySelector(`#${editorId} .line:nth-child(${line})`);
                if (lineElement) {
                    lineElement.classList.add(newState);

                    // Scroll to the middle line of the highlighted range
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

// Given a line in sourceEditorId, find the corresponding lines in the other editors that should be highlighted.
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
                result.generatedCode =  postToCppCode[lineNumber] || [];
            }
            break;

        case 'generatedCode':
            if (codeData) {
                // Python code
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

// Resizable Panels Start
function setupResizablePanels() {
    const container = document.querySelector('.editor-container');
    const pre = document.getElementById('preGradGraph');
    const post = document.getElementById('postGradGraph');
    const code = document.getElementById('generatedCode');
    const divider1 = document.getElementById('divider1');
    const divider2 = document.getElementById('divider2');

    let isDragging = false;
    let dragDivider = null;

    function onMouseMove(e) {
        if (!isDragging || !dragDivider) return;

        const containerRect = container.getBoundingClientRect();
        const totalWidth = containerRect.width;

        if (dragDivider === divider1) {
            const newPreWidth = e.clientX - containerRect.left;
            const newPostWidth = post.offsetWidth + (pre.offsetWidth - newPreWidth);
            pre.style.flex = `0 0 ${newPreWidth}px`;
            post.style.flex = `0 0 ${newPostWidth}px`;
        } else if (dragDivider === divider2) {
            const preWidth = pre.offsetWidth;
            const newPostWidth = e.clientX - containerRect.left - preWidth - divider1.offsetWidth;
            const newCodeWidth = totalWidth - e.clientX + containerRect.left - divider2.offsetWidth;
            post.style.flex = `0 0 ${newPostWidth}px`;
            code.style.flex = `0 0 ${newCodeWidth}px`;
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
        });
    });
}

window.addEventListener('DOMContentLoaded', setupResizablePanels);

// Resizable Panels End
